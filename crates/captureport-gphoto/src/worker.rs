use crate::ffi;
use captureport_core::{
    classify_path, CancellationToken, MediaId, MediaItem, MediaLocator, MediaMetadata, MediaSource,
    MediaStream, MediaType, MetadataState, ScanContext, SourceError, SourceId, SourceIdentity,
    SourceReadProgress, SourceType, TimestampSource,
};
use std::{
    collections::HashMap,
    ffi::CString,
    fs::{File, OpenOptions},
    io::{self, Read, Seek, SeekFrom},
    os::{
        fd::{AsRawFd, IntoRawFd},
        raw::{c_char, c_ulong, c_void},
        unix::fs::OpenOptionsExt,
    },
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GPhotoError {
    pub code: i32,
    pub operation: String,
}
impl std::fmt::Display for GPhotoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} failed (libgphoto2 error {})",
            self.operation, self.code
        )
    }
}
impl std::error::Error for GPhotoError {}
impl From<GPhotoError> for SourceError {
    fn from(e: GPhotoError) -> Self {
        SourceError::Io(format!("Camera access failed during {}. Check the connection and close other camera applications.",e.operation))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CameraDescriptor {
    pub model: String,
    pub port: String,
    pub stable_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Preview {
    pub bytes: Vec<u8>,
    pub mime_type: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WorkerStats {
    pub requests: u64,
}

enum Request {
    Enumerate(CancellationToken, mpsc::Sender<EnumEvent>),
    Read {
        path: String,
        offset: u64,
        length: usize,
        cancellation: CancellationToken,
        reply: mpsc::Sender<ReadEvent>,
    },
    Preview(
        String,
        CancellationToken,
        mpsc::Sender<Result<Preview, GPhotoError>>,
    ),
    Release(String),
    Stats(mpsc::Sender<WorkerStats>),
    Shutdown,
}
enum ReadEvent {
    Progress(SourceReadProgress),
    Done(Result<Vec<u8>, GPhotoError>),
}
enum EnumEvent {
    Item(Box<MediaItem>),
    Done(Result<(), GPhotoError>),
}
#[derive(Clone)]
struct Handle {
    tx: mpsc::Sender<Request>,
}

pub struct GPhotoSource {
    identity: SourceIdentity,
    handle: Handle,
}

impl GPhotoSource {
    /// Detect cameras currently visible to libgphoto2. Detection is a snapshot;
    /// callers can poll this method to observe connect/disconnect events.
    pub fn autodetect() -> Result<Vec<CameraDescriptor>, GPhotoError> {
        unsafe {
            let context = ffi::gp_context_new();
            if context.is_null() {
                return Err(err(-1, "gp_context_new"));
            }
            let mut list = std::ptr::null_mut();
            let rc = ffi::gp_list_new(&mut list);
            if rc < 0 {
                ffi::gp_context_unref(context);
                return Err(err(rc, "gp_list_new"));
            }
            let rc = ffi::gp_camera_autodetect(list, context);
            if rc < 0 {
                ffi::gp_list_free(list);
                ffi::gp_context_unref(context);
                return Err(err(rc, "gp_camera_autodetect"));
            }
            let mut result = Vec::new();
            for i in 0..ffi::gp_list_count(list) {
                let mut model = std::ptr::null();
                let mut port = std::ptr::null();
                if ffi::gp_list_get_name(list, i, &mut model) >= 0
                    && ffi::gp_list_get_value(list, i, &mut port) >= 0
                {
                    let model = ffi::text(model);
                    let port = ffi::text(port);
                    result.push(CameraDescriptor {
                        stable_id: stable_key(&model, &port),
                        model,
                        port,
                    });
                }
            }
            ffi::gp_list_free(list);
            ffi::gp_context_unref(context);
            Ok(result)
        }
    }

    pub fn connect(descriptor: CameraDescriptor) -> Result<Self, GPhotoError> {
        Self::connect_with_cancellation(descriptor, CancellationToken::new())
    }

    pub fn connect_with_cancellation(
        descriptor: CameraDescriptor,
        cancellation: CancellationToken,
    ) -> Result<Self, GPhotoError> {
        if cancellation.is_cancelled() {
            return Err(err(-2, "connection cancelled"));
        }
        let worker_descriptor = descriptor.clone();
        let (tx, rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        thread::Builder::new()
            .name("captureport-gphoto".into())
            .spawn(move || {
                let mut camera =
                    match unsafe { CameraWorker::open(&worker_descriptor, &cancellation) } {
                        Ok(c) => {
                            let _ = ready_tx.send(Ok(()));
                            c
                        }
                        Err(e) => {
                            let _ = ready_tx.send(Err(e));
                            return;
                        }
                    };
                while let Ok(request) = rx.recv() {
                    if !camera.handle(request) {
                        break;
                    }
                }
                camera.close();
            })
            .map_err(|_| err(-1, "spawn gphoto worker"))?;
        ready_rx
            .recv()
            .map_err(|_| err(-1, "gphoto worker startup"))??;
        let source_id = SourceId(hash64(&descriptor.stable_id));
        let identity = SourceIdentity {
            id: source_id,
            source_type: SourceType::Camera,
            stable_id: Some(descriptor.stable_id),
            serial: None,
            manufacturer: None,
            model: Some(descriptor.model.clone()),
            volume_uuid: None,
            display_name: Some(descriptor.model.clone()),
        };
        Ok(Self {
            identity,
            handle: Handle { tx },
        })
    }

    pub fn descriptor_identity(&self) -> &SourceIdentity {
        &self.identity
    }
    pub fn read_preview(&self, locator: &MediaLocator) -> Result<Preview, GPhotoError> {
        call(&self.handle.tx, |reply| {
            Request::Preview(locator.0.clone(), CancellationToken::new(), reply)
        })
    }
    pub fn stats(&self) -> WorkerStats {
        let (reply, result) = mpsc::channel();
        if self.handle.tx.send(Request::Stats(reply)).is_err() {
            return WorkerStats::default();
        }
        result.recv().unwrap_or_default()
    }
}

impl Drop for GPhotoSource {
    fn drop(&mut self) {
        let _ = self.handle.tx.send(Request::Shutdown);
    }
}
impl MediaSource for GPhotoSource {
    fn identity(&self) -> SourceIdentity {
        self.identity.clone()
    }
    fn enumerate(
        &self,
        scan: &ScanContext,
        emit: &mut dyn FnMut(MediaItem) -> Result<(), SourceError>,
    ) -> Result<(), SourceError> {
        if scan.is_cancelled() {
            return Err(SourceError::Cancelled);
        }
        let (events_tx, events_rx) = mpsc::channel();
        self.handle
            .tx
            .send(Request::Enumerate(scan.cancellation().clone(), events_tx))
            .map_err(|_| SourceError::Io("camera disconnected".into()))?;
        while let Ok(event) = events_rx.recv() {
            match event {
                EnumEvent::Item(item) => {
                    if scan.is_cancelled() {
                        return Err(SourceError::Cancelled);
                    }
                    if let Err(error) = emit(*item) {
                        scan.cancellation().cancel();
                        return Err(error);
                    }
                }
                EnumEvent::Done(result) => {
                    return if scan.is_cancelled() {
                        Err(SourceError::Cancelled)
                    } else {
                        result.map_err(|error| SourceError::Io(error.to_string()))
                    };
                }
            }
        }
        if scan.is_cancelled() {
            Err(SourceError::Cancelled)
        } else {
            Err(SourceError::Io(
                "Camera scan ended without a completion result".into(),
            ))
        }
    }
    fn open_stream(&self, item: &MediaLocator) -> Result<Box<dyn Read + Send>, SourceError> {
        Ok(Box::new(CameraReader {
            handle: self.handle.clone(),
            locator: item.clone(),
            offset: 0,
        }))
    }
    fn open_transfer(&self, item: &MediaLocator) -> Result<Box<dyn MediaStream>, SourceError> {
        Ok(Box::new(CameraReader {
            handle: self.handle.clone(),
            locator: item.clone(),
            offset: 0,
        }))
    }
    fn read_range(
        &self,
        item: &MediaLocator,
        offset: u64,
        length: usize,
    ) -> Result<Vec<u8>, SourceError> {
        Ok(camera_read(
            &self.handle,
            &item.0,
            offset,
            length,
            &CancellationToken::new(),
            &mut |_| {},
        )?)
    }

    fn preview(&self, item: &MediaLocator) -> Result<Option<Vec<u8>>, SourceError> {
        self.preview_with_cancellation(item, &CancellationToken::new())
    }
    fn preview_with_cancellation(
        &self,
        item: &MediaLocator,
        cancellation: &CancellationToken,
    ) -> Result<Option<Vec<u8>>, SourceError> {
        if cancellation.is_cancelled() {
            return Err(SourceError::Cancelled);
        }
        let result = call(&self.handle.tx, |reply| {
            Request::Preview(item.0.clone(), cancellation.clone(), reply)
        });
        if cancellation.is_cancelled() {
            return Err(SourceError::Cancelled);
        }
        result
            .map(|preview| Some(preview.bytes))
            .map_err(Into::into)
    }
}

struct CameraReader {
    handle: Handle,
    locator: MediaLocator,
    offset: u64,
}
impl Drop for CameraReader {
    fn drop(&mut self) {
        let _ = self
            .handle
            .tx
            .send(Request::Release(self.locator.0.clone()));
    }
}
impl Read for CameraReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.read_with_progress(buf, &CancellationToken::new(), &mut |_| {})
    }
}
impl MediaStream for CameraReader {
    fn read_with_progress(
        &mut self,
        buf: &mut [u8],
        cancellation: &CancellationToken,
        progress: &mut dyn FnMut(SourceReadProgress),
    ) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let bytes = camera_read(
            &self.handle,
            &self.locator.0,
            self.offset,
            buf.len(),
            cancellation,
            progress,
        )
        .map_err(io::Error::other)?;
        if bytes.len() > buf.len() {
            return Err(io::Error::other("Camera returned too many bytes"));
        }
        let len = bytes.len();
        buf[..len].copy_from_slice(&bytes);
        self.offset += len as u64;
        Ok(len)
    }
}

fn camera_read(
    handle: &Handle,
    path: &str,
    offset: u64,
    length: usize,
    cancellation: &CancellationToken,
    progress: &mut dyn FnMut(SourceReadProgress),
) -> Result<Vec<u8>, GPhotoError> {
    if cancellation.is_cancelled() {
        return Err(err(-2, "transfer cancelled"));
    }
    let (reply, events) = mpsc::channel();
    handle
        .tx
        .send(Request::Read {
            path: path.to_owned(),
            offset,
            length,
            cancellation: cancellation.clone(),
            reply,
        })
        .map_err(|_| err(-1, "camera disconnected"))?;
    while let Ok(event) = events.recv() {
        match event {
            ReadEvent::Progress(value) => progress(value),
            ReadEvent::Done(result) => return result,
        }
    }
    Err(err(-1, "camera disconnected"))
}

fn call<T>(
    tx: &mpsc::Sender<Request>,
    make: impl FnOnce(mpsc::Sender<Result<T, GPhotoError>>) -> Request,
) -> Result<T, GPhotoError> {
    let (reply, result) = mpsc::channel();
    tx.send(make(reply))
        .map_err(|_| err(-1, "camera disconnected"))?;
    result.recv().map_err(|_| err(-1, "camera disconnected"))?
}

struct CameraCallbackState {
    cancellation: CancellationToken,
    reply: Option<mpsc::Sender<ReadEvent>>,
    expected_size: u64,
    target: f32,
    next_id: u32,
    active_id: Option<u32>,
    last_progress: Instant,
}
impl CameraCallbackState {
    fn report(&mut self, current: f32, force: bool) {
        if !current.is_finite() || self.target <= 0. || !self.target.is_finite() {
            return;
        }
        if !force && self.last_progress.elapsed() < Duration::from_millis(100) {
            return;
        }
        if let Some(reply) = &self.reply {
            let fraction = (f64::from(current) / f64::from(self.target)).clamp(0., 1.);
            let _ = reply.send(ReadEvent::Progress(SourceReadProgress {
                bytes: (fraction * self.expected_size as f64) as u64,
                total: self.expected_size,
            }));
        }
        self.last_progress = Instant::now();
    }
}

/// The callbacks' boxed data stays pinned until every C callback is detached.
struct CameraCallbacks {
    context: *mut ffi::GPContext,
    _state: Box<CameraCallbackState>,
}
impl CameraCallbacks {
    fn new(
        context: *mut ffi::GPContext,
        cancellation: CancellationToken,
        reply: Option<mpsc::Sender<ReadEvent>>,
        expected_size: u64,
    ) -> Self {
        let mut state = Box::new(CameraCallbackState {
            cancellation,
            reply,
            expected_size,
            target: 0.,
            next_id: 0,
            active_id: None,
            last_progress: Instant::now(),
        });
        let data = (&mut *state as *mut CameraCallbackState).cast::<c_void>();
        unsafe {
            ffi::gp_context_set_cancel_func(context, Some(camera_cancel), data);
            ffi::gp_context_set_progress_funcs(
                context,
                Some(camera_progress_start),
                Some(camera_progress_update),
                Some(camera_progress_stop),
                data,
            );
        }
        Self {
            context,
            _state: state,
        }
    }
}
impl Drop for CameraCallbacks {
    fn drop(&mut self) {
        unsafe {
            ffi::gp_context_set_cancel_func(self.context, None, std::ptr::null_mut());
            ffi::gp_context_set_progress_funcs(
                self.context,
                None,
                None,
                None,
                std::ptr::null_mut(),
            );
        }
    }
}
unsafe extern "C" fn camera_cancel(_: *mut ffi::GPContext, data: *mut c_void) -> i32 {
    data.cast::<CameraCallbackState>()
        .as_ref()
        .is_some_and(|state| state.cancellation.is_cancelled()) as i32
}
unsafe extern "C" fn camera_progress_start(
    _: *mut ffi::GPContext,
    target: f32,
    _: *const c_char,
    data: *mut c_void,
) -> u32 {
    if let Some(state) = data.cast::<CameraCallbackState>().as_mut() {
        state.next_id = state.next_id.wrapping_add(1).max(1);
        if state.active_id.is_none() {
            state.active_id = Some(state.next_id);
            state.target = target;
            state.report(0., true);
        }
        state.next_id
    } else {
        0
    }
}
unsafe extern "C" fn camera_progress_update(
    _: *mut ffi::GPContext,
    id: u32,
    current: f32,
    data: *mut c_void,
) {
    if let Some(state) = data.cast::<CameraCallbackState>().as_mut() {
        if state.active_id == Some(id) {
            state.report(current, current >= state.target);
        }
    }
}
unsafe extern "C" fn camera_progress_stop(_: *mut ffi::GPContext, id: u32, data: *mut c_void) {
    if let Some(state) = data.cast::<CameraCallbackState>().as_mut() {
        // Stop is also called on failure; only a successful download proves 100%.
        if state.active_id == Some(id) {
            state.active_id = None;
        }
    }
}

struct CameraListGuard(*mut ffi::CameraList);
impl Drop for CameraListGuard {
    fn drop(&mut self) {
        unsafe {
            ffi::gp_list_free(self.0);
        }
    }
}

fn camera_list_names(
    list: &CameraListGuard,
    result: i32,
    operation: &str,
) -> Result<Vec<String>, GPhotoError> {
    if result < 0 {
        return Err(err(result, operation));
    }
    let count = unsafe { ffi::gp_list_count(list.0) };
    if count < 0 {
        return Err(err(count, operation));
    }
    let mut names = Vec::new();
    for index in 0..count {
        let mut name = std::ptr::null();
        let result = unsafe { ffi::gp_list_get_name(list.0, index, &mut name) };
        if result < 0 {
            return Err(err(result, operation));
        }
        if name.is_null() {
            return Err(err(-1, "missing camera entry name"));
        }
        let name = unsafe { std::ffi::CStr::from_ptr(name) }
            .to_str()
            .map_err(|_| err(-1, "camera entry name is not UTF-8"))?;
        if name.is_empty() || matches!(name, "." | "..") || name.contains(['/', '\\']) {
            return Err(err(-1, "unsafe camera entry name"));
        }
        names.push(name.to_owned());
    }
    Ok(names)
}

fn camera_file_size(
    result: i32,
    info: &ffi::CameraFileInfoFile,
    path: &str,
) -> Result<u64, GPhotoError> {
    if result < 0 {
        return Err(err(
            result,
            &format!("read camera file information: {path}"),
        ));
    }
    if info.fields & ffi::FILE_INFO_SIZE == 0 || info.size == 0 || info.size > i64::MAX as u64 {
        return Err(err(
            -1,
            &format!("camera file size is unavailable or invalid: {path}"),
        ));
    }
    Ok(info.size)
}

struct CameraWorker {
    camera: *mut ffi::Camera,
    context: *mut ffi::GPContext,
    requests: u64,
    source_id: SourceId,
    next_id: u64,
    spooled: HashMap<String, File>,
    sizes: HashMap<String, u64>,
}
fn create_camera_spool() -> io::Result<File> {
    let mut nonce = [0; 16];
    File::open("/dev/urandom")?.read_exact(&mut nonce)?;
    let path = std::env::temp_dir().join(format!(
        "captureport-gphoto-{}-{:032x}.bin",
        std::process::id(),
        u128::from_ne_bytes(nonce)
    ));
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)?;
    // Unlink immediately: only our fd can access it, and crashes leave no path.
    std::fs::remove_file(path)?;
    Ok(file)
}
unsafe impl Send for CameraWorker {}
impl CameraWorker {
    unsafe fn open(
        desc: &CameraDescriptor,
        cancellation: &CancellationToken,
    ) -> Result<Self, GPhotoError> {
        let context = ffi::gp_context_new();
        if context.is_null() {
            return Err(err(-1, "gp_context_new"));
        }
        let mut abilities_list: *mut ffi::CameraAbilitiesList = std::ptr::null_mut();
        let mut ports: *mut ffi::GPPortInfoList = std::ptr::null_mut();
        let mut camera: *mut ffi::Camera = std::ptr::null_mut();
        let mut rc = ffi::gp_abilities_list_new(&mut abilities_list);
        if rc < 0 {
            return Err(cleanup_open(
                rc,
                "gp_abilities_list_new",
                camera,
                ports,
                abilities_list,
                context,
            ));
        }
        rc = ffi::gp_abilities_list_load(abilities_list, context);
        if rc < 0 {
            return Err(cleanup_open(
                rc,
                "gp_abilities_list_load",
                camera,
                ports,
                abilities_list,
                context,
            ));
        }
        let model = CString::new(desc.model.as_str()).map_err(|_| err(-1, "camera model"))?;
        let ai = ffi::gp_abilities_list_lookup_model(abilities_list, model.as_ptr());
        if ai < 0 {
            return Err(cleanup_open(
                ai,
                "lookup camera abilities",
                camera,
                ports,
                abilities_list,
                context,
            ));
        }
        let mut abilities = std::mem::zeroed();
        rc = ffi::gp_abilities_list_get_abilities(abilities_list, ai, &mut abilities);
        if rc < 0 {
            return Err(cleanup_open(
                rc,
                "get camera abilities",
                camera,
                ports,
                abilities_list,
                context,
            ));
        }
        rc = ffi::gp_port_info_list_new(&mut ports);
        if rc < 0 {
            return Err(cleanup_open(
                rc,
                "gp_port_info_list_new",
                camera,
                ports,
                abilities_list,
                context,
            ));
        }
        rc = ffi::gp_port_info_list_load(ports);
        if rc < 0 {
            return Err(cleanup_open(
                rc,
                "gp_port_info_list_load",
                camera,
                ports,
                abilities_list,
                context,
            ));
        }
        let port = CString::new(desc.port.as_str()).map_err(|_| err(-1, "camera port"))?;
        let pi = ffi::gp_port_info_list_lookup_path(ports, port.as_ptr());
        if pi < 0 {
            return Err(cleanup_open(
                pi,
                "lookup camera port",
                camera,
                ports,
                abilities_list,
                context,
            ));
        }
        let mut port_info = std::ptr::null_mut();
        rc = ffi::gp_port_info_list_get_info(ports, pi, &mut port_info);
        if rc < 0 {
            return Err(cleanup_open(
                rc,
                "get camera port",
                camera,
                ports,
                abilities_list,
                context,
            ));
        }
        rc = ffi::gp_camera_new(&mut camera);
        if rc < 0 {
            return Err(cleanup_open(
                rc,
                "gp_camera_new",
                camera,
                ports,
                abilities_list,
                context,
            ));
        }
        rc = ffi::gp_camera_set_abilities(camera, abilities);
        if rc < 0 {
            return Err(cleanup_open(
                rc,
                "gp_camera_set_abilities",
                camera,
                ports,
                abilities_list,
                context,
            ));
        }
        rc = ffi::gp_camera_set_port_info(camera, port_info);
        if rc < 0 {
            return Err(cleanup_open(
                rc,
                "gp_camera_set_port_info",
                camera,
                ports,
                abilities_list,
                context,
            ));
        }
        rc = {
            let _callbacks = CameraCallbacks::new(context, cancellation.clone(), None, 0);
            ffi::gp_camera_init(camera, context)
        };
        if rc < 0 {
            return Err(cleanup_open(
                rc,
                "gp_camera_init",
                camera,
                ports,
                abilities_list,
                context,
            ));
        }
        ffi::gp_port_info_list_free(ports);
        ffi::gp_abilities_list_free(abilities_list);
        Ok(Self {
            camera,
            context,
            requests: 0,
            source_id: SourceId(hash64(&stable_key(&desc.model, &desc.port))),
            next_id: 0,
            spooled: HashMap::new(),
            sizes: HashMap::new(),
        })
    }
    fn handle(&mut self, request: Request) -> bool {
        self.requests += 1;
        match request {
            Request::Enumerate(cancel, reply) => {
                let _callbacks = CameraCallbacks::new(self.context, cancel.clone(), None, 0);
                let result = self.enumerate(&cancel, &reply);
                let _ = reply.send(EnumEvent::Done(result));
                true
            }
            Request::Read {
                path,
                offset,
                length,
                cancellation,
                reply,
            } => {
                let _callbacks = CameraCallbacks::new(self.context, cancellation.clone(), None, 0);
                let result = if cancellation.is_cancelled() {
                    Err(err(-2, "transfer cancelled"))
                } else {
                    self.read(&path, offset, length, &cancellation, &reply)
                };
                if result.is_err() {
                    self.spooled.remove(&path);
                }
                let _ = reply.send(ReadEvent::Done(result));
                true
            }
            Request::Preview(path, cancellation, reply) => {
                let _callbacks = CameraCallbacks::new(self.context, cancellation.clone(), None, 0);
                let result = if cancellation.is_cancelled() {
                    Err(err(-2, "preview cancelled"))
                } else {
                    self.preview(&path)
                };
                let _ = reply.send(result);
                true
            }
            Request::Release(path) => {
                self.spooled.remove(&path);
                true
            }
            Request::Stats(reply) => {
                let _ = reply.send(WorkerStats {
                    requests: self.requests,
                });
                true
            }
            Request::Shutdown => false,
        }
    }
    fn enumerate(
        &mut self,
        cancel: &CancellationToken,
        reply: &mpsc::Sender<EnumEvent>,
    ) -> Result<(), GPhotoError> {
        self.spooled.clear();
        self.sizes.clear();
        self.walk("/", cancel, reply)
    }
    fn next_media_id(&mut self) -> u64 {
        self.next_id = self.next_id.saturating_add(1);
        self.next_id
    }
    fn walk(
        &mut self,
        folder: &str,
        cancel: &CancellationToken,
        reply: &mpsc::Sender<EnumEvent>,
    ) -> Result<(), GPhotoError> {
        if cancel.is_cancelled() {
            return Err(err(-2, "enumeration cancelled"));
        }
        let cfolder = CString::new(folder).map_err(|_| err(-1, "folder path"))?;
        for name in self.list_entries(&cfolder, false)? {
            if cancel.is_cancelled() {
                return Err(err(-2, "enumeration cancelled"));
            }
            let path = join_path(folder, &name);
            if !is_camera_media_path(&path) {
                continue;
            }
            let cfile = CString::new(name).map_err(|_| err(-1, "file name"))?;
            let mut info: ffi::CameraFileInfo = unsafe { std::mem::zeroed() };
            let result = unsafe {
                ffi::gp_camera_file_get_info(
                    self.camera,
                    cfolder.as_ptr(),
                    cfile.as_ptr(),
                    &mut info,
                    self.context,
                )
            };
            let size = camera_file_size(result, &info.file, &path)?;
            let mut item =
                MediaItem::new(MediaId(self.next_media_id()), self.source_id, &path, size);
            item.metadata = MetadataState::Ready(camera_metadata(&info.file));
            self.sizes.insert(path, size);
            reply
                .send(EnumEvent::Item(Box::new(item)))
                .map_err(|_| err(-1, "enumeration consumer disconnected"))?;
        }
        if cancel.is_cancelled() {
            return Err(err(-2, "enumeration cancelled"));
        }
        for name in self.list_entries(&cfolder, true)? {
            self.walk(&join_path(folder, &name), cancel, reply)?;
        }
        Ok(())
    }
    fn list_entries(&self, folder: &CString, folders: bool) -> Result<Vec<String>, GPhotoError> {
        let mut list = std::ptr::null_mut();
        let result = unsafe { ffi::gp_list_new(&mut list) };
        if result < 0 {
            return Err(err(result, "allocate camera list"));
        }
        if list.is_null() {
            return Err(err(-1, "allocate camera list"));
        }
        let list = CameraListGuard(list);
        let result = unsafe {
            if folders {
                ffi::gp_camera_folder_list_folders(
                    self.camera,
                    folder.as_ptr(),
                    list.0,
                    self.context,
                )
            } else {
                ffi::gp_camera_folder_list_files(self.camera, folder.as_ptr(), list.0, self.context)
            }
        };
        camera_list_names(
            &list,
            result,
            &format!(
                "list camera {} in {}",
                if folders { "folders" } else { "files" },
                folder.to_string_lossy()
            ),
        )
    }
    fn read(
        &mut self,
        path: &str,
        offset: u64,
        len: usize,
        cancellation: &CancellationToken,
        reply: &mpsc::Sender<ReadEvent>,
    ) -> Result<Vec<u8>, GPhotoError> {
        if self.spooled.contains_key(path) {
            return self.read_from_spool(path, offset, len, cancellation, reply);
        }
        let (folder, file) = split_path(path);
        let folder = CString::new(folder).map_err(|_| err(-1, "folder path"))?;
        let file = CString::new(file).map_err(|_| err(-1, "file path"))?;
        let mut bytes = vec![0u8; len];
        let mut size = len as u64;
        let rc = unsafe {
            ffi::gp_camera_file_read(
                self.camera,
                folder.as_ptr(),
                file.as_ptr(),
                ffi::FILE_NORMAL,
                offset,
                bytes.as_mut_ptr() as *mut c_char,
                &mut size,
                self.context,
            )
        };
        if cancellation.is_cancelled() {
            return Err(err(-2, "transfer cancelled"));
        }
        if rc == ffi::ERROR_NOT_SUPPORTED {
            return self.read_from_spool(path, offset, len, cancellation, reply);
        }
        if rc < 0 {
            return Err(err(rc, "read camera file"));
        }
        if size > len as u64 {
            return Err(err(-1, "camera returned an oversized range"));
        }
        bytes.truncate(size as usize);
        Ok(bytes)
    }
    /// Some drivers reject GP_FILE_READ. Download once into a private fd
    /// backed CameraFile and serve subsequent ranges from that spool. The
    /// fd-backed API keeps the complete file out of a Rust Vec.
    fn read_from_spool(
        &mut self,
        path: &str,
        offset: u64,
        len: usize,
        cancellation: &CancellationToken,
        reply: &mpsc::Sender<ReadEvent>,
    ) -> Result<Vec<u8>, GPhotoError> {
        if cancellation.is_cancelled() {
            return Err(err(-2, "transfer cancelled"));
        }
        if !self.spooled.contains_key(path) {
            let _callbacks = CameraCallbacks::new(
                self.context,
                cancellation.clone(),
                Some(reply.clone()),
                self.sizes.get(path).copied().unwrap_or(0),
            );
            let _ = reply.send(ReadEvent::Progress(SourceReadProgress {
                bytes: 0,
                total: self.sizes.get(path).copied().unwrap_or(0),
            }));
            self.spooled.clear();
            let mut file =
                create_camera_spool().map_err(|e| err(-1, &format!("create camera spool: {e}")))?;
            let (folder, name) = split_path(path);
            let folder = CString::new(folder).map_err(|_| err(-1, "folder path"))?;
            let name = CString::new(name).map_err(|_| err(-1, "file path"))?;
            let mut camera_file = std::ptr::null_mut();
            let duplicate = file
                .try_clone()
                .map_err(|e| err(-1, &format!("clone camera spool: {e}")))?;
            let rc = unsafe { ffi::gp_file_new_from_fd(&mut camera_file, duplicate.as_raw_fd()) };
            if rc < 0 {
                return Err(err(rc, "create fd-backed camera file"));
            }
            let _ = duplicate.into_raw_fd();
            let rc = unsafe {
                ffi::gp_camera_file_get(
                    self.camera,
                    folder.as_ptr(),
                    name.as_ptr(),
                    ffi::FILE_NORMAL,
                    camera_file,
                    self.context,
                )
            };
            unsafe {
                ffi::gp_file_free(camera_file);
            }
            if cancellation.is_cancelled() {
                return Err(err(-2, "transfer cancelled"));
            }
            if rc < 0 {
                return Err(err(rc, "download camera file"));
            }
            if let Some(expected) = self.sizes.get(path) {
                let actual = file
                    .metadata()
                    .map_err(|e| err(-1, &format!("stat camera spool: {e}")))?
                    .len();
                if actual != *expected {
                    return Err(err(-1, &format!("camera file size changed: {path}")));
                }
            }
            file.seek(SeekFrom::Start(0))
                .map_err(|e| err(-1, &format!("seek camera spool: {e}")))?;
            if let Some(&size) = self.sizes.get(path) {
                let _ = reply.send(ReadEvent::Progress(SourceReadProgress {
                    bytes: size,
                    total: size,
                }));
            }
            self.spooled.insert(path.to_owned(), file);
        }
        let spool = self.spooled.get_mut(path).expect("inserted above");
        spool
            .seek(SeekFrom::Start(offset))
            .map_err(|e| err(-1, &format!("seek camera spool: {e}")))?;
        let mut result = vec![0; len];
        let count = spool
            .read(&mut result)
            .map_err(|e| err(-1, &format!("read camera spool: {e}")))?;
        result.truncate(count);
        if count == 0 && offset == 0 && len != 0 {
            return Err(err(-1, "empty camera spool"));
        }
        Ok(result)
    }
    fn preview(&mut self, path: &str) -> Result<Preview, GPhotoError> {
        let (folder, file) = split_path(path);
        unsafe {
            let folder = CString::new(folder).map_err(|_| err(-1, "folder path"))?;
            let file = CString::new(file).map_err(|_| err(-1, "file path"))?;
            let mut output = std::ptr::null_mut();
            let mut rc = ffi::gp_file_new(&mut output);
            if rc < 0 {
                return Err(err(rc, "gp_file_new"));
            }
            rc = ffi::gp_camera_file_get(
                self.camera,
                folder.as_ptr(),
                file.as_ptr(),
                ffi::FILE_PREVIEW,
                output,
                self.context,
            );
            if rc < 0 {
                ffi::gp_file_free(output);
                return Err(err(rc, "read camera preview"));
            }
            let mut data = std::ptr::null();
            let mut size: c_ulong = 0;
            rc = ffi::gp_file_get_data_and_size(output, &mut data, &mut size);
            let bytes = if rc >= 0 && !data.is_null() {
                std::slice::from_raw_parts(data as *const u8, size as usize).to_vec()
            } else {
                Vec::new()
            };
            ffi::gp_file_free(output);
            if rc < 0 {
                return Err(err(rc, "get preview bytes"));
            }
            Ok(Preview {
                bytes,
                mime_type: None,
            })
        }
    }
    fn close(&mut self) {
        self.spooled.clear();
        unsafe {
            let _ = ffi::gp_camera_exit(self.camera, self.context);
            let _ = ffi::gp_camera_free(self.camera);
            ffi::gp_context_unref(self.context);
        }
    }
}

fn split_path(path: &str) -> (&str, &str) {
    path.rsplit_once('/')
        .map(|(f, n)| (if f.is_empty() { "/" } else { f }, n))
        .unwrap_or(("/", path))
}
fn camera_metadata(info: &ffi::CameraFileInfoFile) -> MediaMetadata {
    // Zero is commonly used for an unavailable PTP date; never invent a capture time.
    let capture_time = (info.fields & ffi::FILE_INFO_MTIME != 0 && info.mtime > 0)
        .then(|| chrono::DateTime::from_timestamp(info.mtime, 0))
        .flatten()
        .map(|time| time.to_rfc3339());
    let timestamp_source = capture_time.as_ref().map(|_| TimestampSource::Camera);
    MediaMetadata {
        capture_time,
        timestamp_source,
        ..MediaMetadata::default()
    }
}

fn is_camera_media_path(path: &str) -> bool {
    !matches!(classify_path(path), MediaType::Sidecar | MediaType::Unknown)
}
fn join_path(folder: &str, name: &str) -> String {
    if folder == "/" {
        format!("/{name}")
    } else {
        format!("{folder}/{name}")
    }
}
fn stable_key(model: &str, port: &str) -> String {
    format!("gphoto:{:016x}", hash64(&format!("{model}\0{port}")))
}
fn hash64(value: &str) -> u64 {
    let mut hash = 1469598103934665603u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(1099511628211);
    }
    hash
}
fn err(code: i32, operation: &str) -> GPhotoError {
    GPhotoError {
        code,
        operation: operation.into(),
    }
}
unsafe fn cleanup_open(
    code: i32,
    operation: &str,
    camera: *mut ffi::Camera,
    ports: *mut ffi::GPPortInfoList,
    abilities: *mut ffi::CameraAbilitiesList,
    context: *mut ffi::GPContext,
) -> GPhotoError {
    if !camera.is_null() {
        let _ = ffi::gp_camera_free(camera);
    }
    if !ports.is_null() {
        let _ = ffi::gp_port_info_list_free(ports);
    }
    if !abilities.is_null() {
        let _ = ffi::gp_abilities_list_free(abilities);
    }
    if !context.is_null() {
        ffi::gp_context_unref(context);
    }
    err(code, operation)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_failures_and_unsafe_entries_are_not_empty_successes() {
        for name in ["DSCF4016.MOV", "..", "/DCIM", "a/b", "a\\b", ""] {
            let mut list = std::ptr::null_mut();
            assert_eq!(unsafe { ffi::gp_list_new(&mut list) }, 0);
            let list = CameraListGuard(list);
            assert!(camera_list_names(&list, -7, "list camera folders in /DCIM")
                .unwrap_err()
                .operation
                .contains("/DCIM"));
            let name_c = CString::new(name).unwrap();
            assert!(unsafe { ffi::gp_list_append(list.0, name_c.as_ptr(), std::ptr::null()) } >= 0);
            let result = camera_list_names(&list, 0, "list camera files");
            if name == "DSCF4016.MOV" {
                assert_eq!(result.unwrap(), vec![name]);
            } else {
                assert!(result.is_err(), "accepted {name:?}");
            }
        }
    }

    #[test]
    fn file_size_requires_success_a_valid_flag_and_a_supported_value() {
        let mut info: ffi::CameraFileInfoFile = unsafe { std::mem::zeroed() };
        info.fields = ffi::FILE_INFO_SIZE;
        info.size = 5_000_000_000;
        assert_eq!(
            camera_file_size(0, &info, "large.MOV").unwrap(),
            5_000_000_000
        );
        assert!(camera_file_size(-7, &info, "large.MOV")
            .unwrap_err()
            .operation
            .contains("large.MOV"));
        info.fields = 0;
        assert!(camera_file_size(0, &info, "large.MOV").is_err());
        info.fields = ffi::FILE_INFO_SIZE;
        for invalid in [0, u64::MAX] {
            info.size = invalid;
            assert!(camera_file_size(0, &info, "large.MOV").is_err());
        }
    }

    #[test]
    fn disconnected_enumeration_without_done_is_an_error() {
        let (tx, rx) = mpsc::channel();
        let worker = thread::spawn(move || {
            let Request::Enumerate(_, reply) = rx.recv().unwrap() else {
                panic!("expected enumeration");
            };
            drop(reply);
        });
        let source = GPhotoSource {
            identity: captureport_core::FakeSourceBuilder::new()
                .build()
                .identity(),
            handle: Handle { tx },
        };
        let scan = ScanContext::new(
            captureport_core::ScanGeneration(1),
            CancellationToken::new(),
        );
        assert!(source
            .enumerate(&scan, &mut |_| Ok(()))
            .unwrap_err()
            .to_string()
            .contains("completion"));
        worker.join().unwrap();
    }

    #[test]
    fn cancelled_connection_does_not_open_hardware() {
        let cancellation = CancellationToken::new();
        cancellation.cancel();
        let result = GPhotoSource::connect_with_cancellation(
            CameraDescriptor {
                model: "test".into(),
                port: "test".into(),
                stable_id: "test".into(),
            },
            cancellation,
        );
        assert!(matches!(result, Err(GPhotoError { code: -2, .. })));
    }

    #[test]
    fn spool_is_private_anonymous_and_independent_per_download() {
        use std::{io::Write, os::unix::fs::MetadataExt};
        let mut first = create_camera_spool().unwrap();
        let second = create_camera_spool().unwrap();
        let metadata = first.metadata().unwrap();
        assert_eq!(metadata.mode() & 0o777, 0o600);
        assert_eq!(metadata.nlink(), 0);
        assert_ne!(metadata.ino(), second.metadata().unwrap().ino());
        first.write_all(b"video").unwrap();
        first.seek(SeekFrom::Start(0)).unwrap();
        let mut bytes = Vec::new();
        first.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"video");
    }

    #[test]
    fn cached_reads_and_stream_drop_release_spool_without_camera_io() {
        use std::io::Write;
        let mut file = create_camera_spool().unwrap();
        file.write_all(b"video").unwrap();
        // This test only takes the cached-read and Release paths, never FFI.
        let mut worker = CameraWorker {
            camera: std::ptr::null_mut(),
            context: std::ptr::null_mut(),
            requests: 0,
            source_id: SourceId(1),
            next_id: 0,
            spooled: HashMap::from([("clip.MOV".into(), file)]),
            sizes: HashMap::new(),
        };
        let (reply, _) = mpsc::channel();
        assert_eq!(
            worker
                .read("clip.MOV", 1, 3, &CancellationToken::new(), &reply)
                .unwrap(),
            b"ide"
        );
        let (tx, rx) = mpsc::channel();
        drop(CameraReader {
            handle: Handle { tx },
            locator: MediaLocator("clip.MOV".into()),
            offset: 0,
        });
        assert!(worker.handle(rx.recv().unwrap()));
        assert!(worker.spooled.is_empty());
    }

    #[test]
    fn context_cancellation_is_detached_before_callback_data_drops() {
        unsafe {
            let context = ffi::gp_context_new();
            assert!(!context.is_null());
            let cancellation = CancellationToken::new();
            let callbacks = CameraCallbacks::new(context, cancellation.clone(), None, 0);
            assert_eq!(ffi::gp_context_cancel(context), 0);
            cancellation.cancel();
            assert_eq!(ffi::gp_context_cancel(context), 1);
            drop(callbacks);
            assert_eq!(ffi::gp_context_cancel(context), 0);
            ffi::gp_context_unref(context);
        }
    }

    #[test]
    fn camera_reader_reports_download_before_returning_bytes() {
        let (tx, rx) = mpsc::channel();
        let worker = thread::spawn(move || {
            let Request::Read {
                reply,
                cancellation,
                ..
            } = rx.recv().unwrap()
            else {
                panic!("expected read");
            };
            assert!(!cancellation.is_cancelled());
            reply
                .send(ReadEvent::Progress(SourceReadProgress {
                    bytes: 5,
                    total: 10,
                }))
                .unwrap();
            reply.send(ReadEvent::Done(Ok(vec![1, 2, 3]))).unwrap();
        });
        let mut reader = CameraReader {
            handle: Handle { tx },
            locator: MediaLocator("clip.MOV".into()),
            offset: 0,
        };
        let mut events = Vec::new();
        let mut buffer = [0; 8];
        assert_eq!(
            reader
                .read_with_progress(&mut buffer, &CancellationToken::new(), &mut |p| events
                    .push(p.bytes))
                .unwrap(),
            3
        );
        assert_eq!(events, vec![5]);
        assert_eq!(&buffer[..3], &[1, 2, 3]);
        assert_eq!(reader.offset, 3);
        worker.join().unwrap();
    }

    #[test]
    fn camera_callbacks_normalize_driver_units_and_observe_cancellation() {
        let (reply, events) = mpsc::channel();
        let cancellation = CancellationToken::new();
        let mut state = CameraCallbackState {
            cancellation: cancellation.clone(),
            reply: Some(reply),
            expected_size: 1000,
            target: 100.,
            next_id: 0,
            active_id: None,
            last_progress: Instant::now() - Duration::from_secs(1),
        };
        state.report(50., false);
        let ReadEvent::Progress(p) = events.recv().unwrap() else {
            panic!("expected progress");
        };
        assert_eq!((p.bytes, p.total), (500, 1000));
        state.report(f32::NAN, true);
        assert!(events.try_recv().is_err());
        let data = (&mut state as *mut CameraCallbackState).cast();
        assert_eq!(unsafe { camera_cancel(std::ptr::null_mut(), data) }, 0);
        cancellation.cancel();
        assert_eq!(unsafe { camera_cancel(std::ptr::null_mut(), data) }, 1);
    }

    #[test]
    fn nested_progress_does_not_replace_the_object_total_or_imply_success() {
        let (reply, events) = mpsc::channel();
        let mut state = CameraCallbackState {
            cancellation: CancellationToken::new(),
            reply: Some(reply),
            expected_size: 1000,
            target: 0.,
            next_id: 0,
            active_id: None,
            last_progress: Instant::now(),
        };
        let data = (&mut state as *mut CameraCallbackState).cast();
        unsafe {
            let outer = camera_progress_start(std::ptr::null_mut(), 100., std::ptr::null(), data);
            let inner = camera_progress_start(std::ptr::null_mut(), 1., std::ptr::null(), data);
            assert_ne!(outer, inner);
            let _ = events.recv().unwrap();
            camera_progress_update(std::ptr::null_mut(), inner, 1., data);
            camera_progress_stop(std::ptr::null_mut(), inner, data);
            assert!(events.try_recv().is_err());
            state.last_progress = Instant::now() - Duration::from_secs(1);
            camera_progress_update(std::ptr::null_mut(), outer, 50., data);
            let ReadEvent::Progress(value) = events.recv().unwrap() else {
                panic!("expected progress");
            };
            assert_eq!(value.bytes, 500);
            camera_progress_stop(std::ptr::null_mut(), outer, data);
            assert!(events.try_recv().is_err());
        }
    }

    #[test]
    fn camera_timestamp_requires_valid_flag_and_value() {
        let mut info: ffi::CameraFileInfoFile = unsafe { std::mem::zeroed() };
        info.fields = ffi::FILE_INFO_MTIME;
        info.mtime = 1_767_225_600;
        let metadata = camera_metadata(&info);
        assert_eq!(
            metadata.capture_time.as_deref(),
            Some("2026-01-01T00:00:00+00:00")
        );
        assert_eq!(metadata.timestamp_source, Some(TimestampSource::Camera));
        info.fields = 0;
        assert_eq!(camera_metadata(&info), MediaMetadata::default());
        info.fields = ffi::FILE_INFO_MTIME;
        for invalid in [0, -1, i64::MAX] {
            info.mtime = invalid;
            assert_eq!(camera_metadata(&info), MediaMetadata::default());
        }
    }

    #[test]
    fn stable_key_is_repeatable() {
        assert_eq!(
            stable_key("Camera", "usb:1,2"),
            stable_key("Camera", "usb:1,2")
        );
        assert_ne!(
            stable_key("Camera", "usb:1,2"),
            stable_key("Other", "usb:1,2")
        );
    }
    #[test]
    fn paths_split_at_last_separator() {
        assert_eq!(split_path("/DCIM/100/file.JPG"), ("/DCIM/100", "file.JPG"));
        assert_eq!(split_path("/file.JPG"), ("/", "file.JPG"));
    }
    #[test]
    fn camera_media_filter_uses_type_not_dcim_location() {
        for path in [
            "/DCIM/100/IMG.JPG",
            "/Pictures/Screenshots/shot.PNG",
            "/Movies/clip.MP4",
            "/Movies/old.3GP",
            "/Download/raw.DNG",
        ] {
            assert!(is_camera_media_path(path), "{path}");
        }
        for path in [
            "/DCIM/100/readme.txt",
            "/Documents/report.pdf",
            "/Pictures/metadata.XMP",
            "/Download/archive.zip",
        ] {
            assert!(!is_camera_media_path(path), "{path}");
        }
    }
}

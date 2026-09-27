use crate::ffi;
use captureport_core::{
    classify_path, CancellationToken, MediaId, MediaItem, MediaLocator, MediaSource, MediaType,
    ScanContext, SourceError, SourceId, SourceIdentity, SourceType,
};
use std::{
    collections::HashMap,
    ffi::CString,
    fs::{File, OpenOptions},
    io::{self, Read, Seek, SeekFrom},
    os::{
        fd::IntoRawFd,
        raw::{c_char, c_ulong},
    },
    path::PathBuf,
    sync::mpsc,
    thread,
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
    Read(
        String,
        u64,
        usize,
        mpsc::Sender<Result<Vec<u8>, GPhotoError>>,
    ),
    Preview(String, mpsc::Sender<Result<Preview, GPhotoError>>),
    Stats(mpsc::Sender<WorkerStats>),
    Shutdown,
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
        let worker_descriptor = descriptor.clone();
        let (tx, rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        thread::Builder::new()
            .name("captureport-gphoto".into())
            .spawn(move || {
                let mut camera = match unsafe { CameraWorker::open(&worker_descriptor) } {
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
            Request::Preview(locator.0.clone(), reply)
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
                    emit(*item)?;
                }
                EnumEvent::Done(result) => {
                    if let Err(error) = result {
                        return if scan.is_cancelled() {
                            Err(SourceError::Cancelled)
                        } else {
                            Err(SourceError::Io(error.to_string()))
                        };
                    }
                    break;
                }
            }
        }
        if scan.is_cancelled() {
            Err(SourceError::Cancelled)
        } else {
            Ok(())
        }
    }
    fn open_stream(&self, item: &MediaLocator) -> Result<Box<dyn Read + Send>, SourceError> {
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
        Ok(call(&self.handle.tx, |reply| {
            Request::Read(item.0.clone(), offset, length, reply)
        })?)
    }

    fn preview(&self, item: &MediaLocator) -> Result<Option<Vec<u8>>, SourceError> {
        self.read_preview(item)
            .map(|preview| Some(preview.bytes))
            .map_err(Into::into)
    }
}

struct CameraReader {
    handle: Handle,
    locator: MediaLocator,
    offset: u64,
}
impl Read for CameraReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let bytes = call(&self.handle.tx, |reply| {
            Request::Read(self.locator.0.clone(), self.offset, buf.len(), reply)
        })
        .map_err(io::Error::other)?;
        let len = bytes.len();
        buf[..len].copy_from_slice(&bytes);
        self.offset += len as u64;
        Ok(len)
    }
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

struct CameraWorker {
    camera: *mut ffi::Camera,
    context: *mut ffi::GPContext,
    requests: u64,
    source_id: SourceId,
    next_id: u64,
    spooled: HashMap<String, Spool>,
}
struct Spool {
    file: File,
    path: PathBuf,
}
impl Drop for Spool {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}
unsafe impl Send for CameraWorker {}
impl CameraWorker {
    unsafe fn open(desc: &CameraDescriptor) -> Result<Self, GPhotoError> {
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
        rc = ffi::gp_camera_init(camera, context);
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
        })
    }
    fn handle(&mut self, request: Request) -> bool {
        self.requests += 1;
        match request {
            Request::Enumerate(cancel, reply) => {
                let result = self.enumerate(&cancel, &reply);
                let _ = reply.send(EnumEvent::Done(result));
                true
            }
            Request::Read(path, offset, len, reply) => {
                let _ = reply.send(self.read(&path, offset, len));
                true
            }
            Request::Preview(path, reply) => {
                let _ = reply.send(self.preview(&path));
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
        unsafe {
            let mut files = std::ptr::null_mut();
            let mut folders = std::ptr::null_mut();
            let cfolder = CString::new(folder).map_err(|_| err(-1, "folder path"))?;
            let rc = ffi::gp_list_new(&mut files);
            if rc < 0 {
                return Err(err(rc, "gp_list_new files"));
            }
            let rc = ffi::gp_camera_folder_list_files(
                self.camera,
                cfolder.as_ptr(),
                files,
                self.context,
            );
            if rc < 0 {
                ffi::gp_list_free(files);
                return Err(err(rc, "list camera files"));
            }
            for i in 0..ffi::gp_list_count(files) {
                if cancel.is_cancelled() {
                    ffi::gp_list_free(files);
                    return Err(err(-2, "enumeration cancelled"));
                }
                let mut name = std::ptr::null();
                if ffi::gp_list_get_name(files, i, &mut name) < 0 {
                    continue;
                }
                let name = ffi::text(name);
                let path = join_path(folder, &name);
                if !is_camera_media_path(&path) {
                    continue;
                }
                let cfile = CString::new(name.as_str()).map_err(|_| err(-1, "file name"))?;
                let mut info = std::mem::zeroed();
                let _ = ffi::gp_camera_file_get_info(
                    self.camera,
                    cfolder.as_ptr(),
                    cfile.as_ptr(),
                    &mut info,
                    self.context,
                );
                let item = MediaItem::new(
                    MediaId(self.next_media_id()),
                    self.source_id,
                    &path,
                    info.file.size,
                );
                if reply.send(EnumEvent::Item(Box::new(item))).is_err() {
                    ffi::gp_list_free(files);
                    return Err(err(-1, "enumeration consumer disconnected"));
                }
            }
            ffi::gp_list_free(files);
            if ffi::gp_list_new(&mut folders) < 0 {
                return Ok(());
            }
            let rc = ffi::gp_camera_folder_list_folders(
                self.camera,
                cfolder.as_ptr(),
                folders,
                self.context,
            );
            if rc >= 0 {
                for i in 0..ffi::gp_list_count(folders) {
                    let mut name = std::ptr::null();
                    if ffi::gp_list_get_name(folders, i, &mut name) >= 0 {
                        let child = join_path(folder, &ffi::text(name));
                        if let Err(e) = self.walk(&child, cancel, reply) {
                            ffi::gp_list_free(folders);
                            return Err(e);
                        }
                    }
                }
            }
            ffi::gp_list_free(folders);
            Ok(())
        }
    }
    fn read(&mut self, path: &str, offset: u64, len: usize) -> Result<Vec<u8>, GPhotoError> {
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
        if rc < 0 {
            return self.read_from_spool(path, offset, len, rc);
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
        range_error: i32,
    ) -> Result<Vec<u8>, GPhotoError> {
        if !self.spooled.contains_key(path) {
            let safe = format!(
                "/tmp/captureport-gphoto-{}-{:016x}.bin",
                std::process::id(),
                hash64(path)
            );
            let mut file = OpenOptions::new()
                .read(true)
                .write(true)
                .create_new(true)
                .open(&safe)
                .map_err(|e| err(-1, &format!("create camera spool: {e}")))?;
            let (folder, name) = split_path(path);
            let folder = CString::new(folder).map_err(|_| err(-1, "folder path"))?;
            let name = CString::new(name).map_err(|_| err(-1, "file path"))?;
            let mut camera_file = std::ptr::null_mut();
            let duplicate_fd = file
                .try_clone()
                .map_err(|e| err(-1, &format!("clone camera spool: {e}")))?
                .into_raw_fd();
            let rc = unsafe { ffi::gp_file_new_from_fd(&mut camera_file, duplicate_fd) };
            if rc < 0 {
                let _ = std::fs::remove_file(&safe);
                return Err(err(rc, "create fd-backed camera file"));
            }
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
            if rc < 0 {
                let _ = std::fs::remove_file(&safe);
                return Err(err(rc, "download camera file"));
            }
            file.seek(SeekFrom::Start(0))
                .map_err(|e| err(-1, &format!("seek camera spool: {e}")))?;
            self.spooled.insert(
                path.to_owned(),
                Spool {
                    file,
                    path: PathBuf::from(safe),
                },
            );
        }
        let spool = self.spooled.get_mut(path).expect("inserted above");
        spool
            .file
            .seek(SeekFrom::Start(offset))
            .map_err(|e| err(-1, &format!("seek camera spool: {e}")))?;
        let mut result = vec![0; len];
        let count = spool
            .file
            .read(&mut result)
            .map_err(|e| err(-1, &format!("read camera spool: {e}")))?;
        result.truncate(count);
        if count == 0 && offset == 0 && len != 0 {
            return Err(err(range_error, "read camera file and spool"));
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

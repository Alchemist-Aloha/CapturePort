use captureport_core::MediaType;
use exif::{In, Reader, Tag, Value};
use image::{DynamicImage, ImageFormat};
use std::sync::atomic::{AtomicBool, Ordering};
use std::{
    fs,
    io::Cursor,
    path::{Path, PathBuf},
    sync::{
        Arc, Condvar, Mutex,
        mpsc::{self, Receiver},
    },
    thread,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ThumbnailFormat {
    Jpeg,
}

#[derive(Clone, Debug)]
pub struct ThumbnailRequest {
    pub source_id: String,
    pub relative_path: String,
    pub size: u64,
    pub modified_unix: u64,
    pub path: PathBuf,
    pub media_type: MediaType,
    pub priority: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ThumbnailState {
    Ready,
    Placeholder,
    Cancelled,
    Failed,
}

#[derive(Clone, Debug)]
pub struct ThumbnailResult {
    pub key: String,
    pub state: ThumbnailState,
    pub bytes: Option<Vec<u8>>,
    pub error: Option<String>,
}

#[derive(Debug)]
pub enum ThumbnailError {
    Cache(String),
    WorkerClosed,
    QueueFull,
}
impl std::fmt::Display for ThumbnailError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ThumbnailError {}

/// A bounded thumbnail queue. Each worker owns decoding state and receives at most
/// `capacity` outstanding jobs, preventing thousands of offscreen decodes.
pub struct ThumbnailPipeline {
    queue: Arc<JobQueue>,
    rx: Receiver<ThumbnailResult>,
    cancelled: Arc<AtomicBool>,
    _workers: Vec<thread::JoinHandle<()>>,
    cache_dir: PathBuf,
}
struct Job {
    request: ThumbnailRequest,
    cache_dir: PathBuf,
    cancelled: Arc<AtomicBool>,
}
struct QueuedJob {
    priority: u8,
    sequence: u64,
    job: Job,
}
struct QueueState {
    jobs: Vec<QueuedJob>,
    next_sequence: u64,
    closed: bool,
    capacity: usize,
}
struct JobQueue {
    state: Mutex<QueueState>,
    wake: Condvar,
}

impl JobQueue {
    fn new(capacity: usize) -> Self {
        Self {
            state: Mutex::new(QueueState {
                jobs: Vec::new(),
                next_sequence: 0,
                closed: false,
                capacity: capacity.max(1),
            }),
            wake: Condvar::new(),
        }
    }
    fn try_push(&self, job: Job) -> Result<(), ThumbnailError> {
        let mut state = self.state.lock().expect("thumbnail queue");
        if state.closed {
            return Err(ThumbnailError::WorkerClosed);
        }
        if state.jobs.len() >= state.capacity {
            return Err(ThumbnailError::QueueFull);
        }
        let priority = job.request.priority;
        let sequence = state.next_sequence;
        state.next_sequence = state.next_sequence.wrapping_add(1);
        state.jobs.push(QueuedJob {
            priority,
            sequence,
            job,
        });
        self.wake.notify_one();
        Ok(())
    }
    fn pop(&self) -> Option<Job> {
        let mut state = self.state.lock().expect("thumbnail queue");
        loop {
            if let Some(index) = state
                .jobs
                .iter()
                .enumerate()
                .max_by_key(|(_, item)| (item.priority, std::cmp::Reverse(item.sequence)))
                .map(|(index, _)| index)
            {
                return Some(state.jobs.swap_remove(index).job);
            }
            if state.closed {
                return None;
            }
            state = self.wake.wait(state).expect("thumbnail queue");
        }
    }
    fn close(&self) {
        let mut state = self.state.lock().expect("thumbnail queue");
        state.closed = true;
        self.wake.notify_all();
    }
}

impl ThumbnailPipeline {
    pub fn new(
        cache_dir: impl AsRef<Path>,
        workers: usize,
        capacity: usize,
    ) -> Result<Self, ThumbnailError> {
        let cache_dir = cache_dir.as_ref().to_path_buf();
        fs::create_dir_all(&cache_dir).map_err(|e| ThumbnailError::Cache(e.to_string()))?;
        let (tx_results, rx) = mpsc::channel();
        let cancelled = Arc::new(AtomicBool::new(false));
        let queue = Arc::new(JobQueue::new(capacity));
        let mut handles = Vec::new();
        for _ in 0..workers.max(1) {
            let jobs = Arc::clone(&queue);
            let output = tx_results.clone();
            let stop = Arc::clone(&cancelled);
            handles.push(thread::spawn(move || {
                while let Some(job) = jobs.pop() {
                    if stop.load(Ordering::Relaxed) || job.cancelled.load(Ordering::Relaxed) {
                        let _ = output.send(ThumbnailResult {
                            key: cache_key(&job.request),
                            state: ThumbnailState::Cancelled,
                            bytes: None,
                            error: None,
                        });
                        continue;
                    }
                    let result = render(job);
                    let _ = output.send(result);
                }
            }));
        }
        Ok(Self {
            queue,
            rx,
            cancelled,
            _workers: handles,
            cache_dir,
        })
    }
    pub fn submit(&self, request: ThumbnailRequest) -> Result<(), ThumbnailError> {
        self.queue.try_push(Job {
            request,
            cache_dir: self.cache_dir.clone(),
            cancelled: Arc::clone(&self.cancelled),
        })
    }
    pub fn try_recv(&self) -> Option<ThumbnailResult> {
        self.rx.try_recv().ok()
    }
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
    pub fn reset_cancellation(&self) {
        self.cancelled.store(false, Ordering::Relaxed);
    }
    pub fn cache_key(request: &ThumbnailRequest) -> String {
        cache_key(request)
    }
}

impl Drop for ThumbnailPipeline {
    fn drop(&mut self) {
        self.queue.close();
        for worker in self._workers.drain(..) {
            let _ = worker.join();
        }
    }
}

fn render(job: Job) -> ThumbnailResult {
    let key = cache_key(&job.request);
    let target = job.cache_dir.join(format!("{key}.jpg"));
    if let Ok(bytes) = fs::read(&target) {
        return ThumbnailResult {
            key,
            state: ThumbnailState::Ready,
            bytes: Some(bytes),
            error: None,
        };
    }
    if matches!(
        job.request.media_type,
        MediaType::Heif | MediaType::Sidecar | MediaType::Unknown
    ) {
        return ThumbnailResult {
            key,
            state: ThumbnailState::Placeholder,
            bytes: None,
            error: Some("no embedded preview or supported decoder".into()),
        };
    }
    if job.request.media_type == MediaType::Video {
        return match ffmpeg_frame(&job.request.path, &job.cancelled) {
            Ok(bytes) => {
                let _ = fs::write(&target, &bytes);
                ThumbnailResult {
                    key,
                    state: ThumbnailState::Ready,
                    bytes: Some(bytes),
                    error: None,
                }
            }
            Err(error) if error == "cancelled" => ThumbnailResult {
                key,
                state: ThumbnailState::Cancelled,
                bytes: None,
                error: None,
            },
            Err(error) => ThumbnailResult {
                key,
                state: ThumbnailState::Placeholder,
                bytes: None,
                error: Some(error),
            },
        };
    }
    if job.request.media_type == MediaType::Raw {
        if let Some(preview) = embedded_preview(&job.request.path)
            && let Ok(bytes) = image::load_from_memory(&preview).and_then(encode)
        {
            let _ = fs::write(&target, &bytes);
            return ThumbnailResult {
                key,
                state: ThumbnailState::Ready,
                bytes: Some(bytes),
                error: None,
            };
        }
        return ThumbnailResult {
            key,
            state: ThumbnailState::Placeholder,
            bytes: None,
            error: Some("RAW has no embedded JPEG preview".into()),
        };
    }
    match image::open(&job.request.path).and_then(encode) {
        Ok(bytes) => {
            let _ = fs::write(&target, &bytes);
            ThumbnailResult {
                key,
                state: ThumbnailState::Ready,
                bytes: Some(bytes),
                error: None,
            }
        }
        Err(error) => ThumbnailResult {
            key,
            state: ThumbnailState::Failed,
            bytes: None,
            error: Some(error.to_string()),
        },
    }
}

fn embedded_preview(path: &Path) -> Option<Vec<u8>> {
    let file = fs::File::open(path).ok()?;
    let mut reader = std::io::BufReader::new(file);
    let exif = Reader::new().read_from_container(&mut reader).ok()?;
    exif_thumbnail(&exif).or_else(|| largest_embedded_jpeg(exif.buf()))
}

/// The EXIF thumbnail pointer (`JPEGInterchangeFormat` in IFD1) used by camera
/// JPEGs and by RAW formats that follow the EXIF layout (ARW, CR2, NEF, ...).
fn exif_thumbnail(exif: &exif::Exif) -> Option<Vec<u8>> {
    let offset = exif
        .get_field(Tag::JPEGInterchangeFormat, In::THUMBNAIL)
        .and_then(|f| match f.value {
            Value::Long(ref values) => values.first().copied(),
            Value::Short(ref values) => values.first().copied().map(u32::from),
            _ => None,
        })? as usize;
    let length = exif
        .get_field(Tag::JPEGInterchangeFormatLength, In::THUMBNAIL)
        .and_then(|f| match f.value {
            Value::Long(ref values) => values.first().copied(),
            Value::Short(ref values) => values.first().copied().map(u32::from),
            _ => None,
        })? as usize;
    let end = offset.checked_add(length)?;
    let data = exif.buf().get(offset..end)?;
    (data.starts_with(&[0xff, 0xd8]) && data.ends_with(&[0xff, 0xd9])).then(|| data.to_vec())
}

/// Some RAW containers (notably DNG) store their preview as a JPEG-compressed
/// TIFF strip instead of an EXIF thumbnail pointer, so fall back to the largest
/// self-contained JPEG stream in the container. The preview is downscaled to a
/// thumbnail anyway, so decode work stays bounded.
fn largest_embedded_jpeg(data: &[u8]) -> Option<Vec<u8>> {
    let mut best: Option<&[u8]> = None;
    let mut cursor = 0;
    while let Some(relative) = data[cursor..]
        .windows(3)
        .position(|window| window == [0xff, 0xd8, 0xff])
    {
        let start = cursor + relative;
        let Some(relative_end) = data[start + 3..]
            .windows(2)
            .position(|window| window == [0xff, 0xd9])
        else {
            break;
        };
        let end = start + 3 + relative_end + 2;
        let candidate = &data[start..end];
        if best.is_none_or(|current| candidate.len() > current.len()) {
            best = Some(candidate);
        }
        cursor = end;
    }
    best.map(|bytes| bytes.to_vec())
}

fn ffmpeg_frame(path: &Path, cancelled: &AtomicBool) -> Result<Vec<u8>, String> {
    let mut child = std::process::Command::new("ffmpeg")
        .args(["-v", "error", "-ss", "0", "-i"])
        .arg(path)
        .args([
            "-frames:v",
            "1",
            "-vf",
            "scale=320:-1",
            "-f",
            "image2pipe",
            "-vcodec",
            "mjpeg",
            "pipe:1",
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("ffmpeg unavailable: {e}"))?;
    let stdout = child.stdout.take().ok_or("ffmpeg output unavailable")?;
    let reader = std::thread::spawn(move || -> Result<(Vec<u8>, bool), String> {
        use std::io::Read;
        let mut output = Vec::new();
        let mut input = stdout;
        let mut buffer = [0u8; 8192];
        let mut exceeded = false;
        loop {
            let count = input.read(&mut buffer).map_err(|error| error.to_string())?;
            if count == 0 {
                break;
            }
            let remaining = (2usize * 1024 * 1024).saturating_sub(output.len());
            output.extend_from_slice(&buffer[..count.min(remaining)]);
            if count > remaining {
                exceeded = true;
            }
        }
        Ok((output, exceeded))
    });
    let start = std::time::Instant::now();
    loop {
        if cancelled.load(Ordering::Relaxed) {
            let _ = child.kill();
            let _ = child.wait();
            let _ = reader.join();
            return Err("cancelled".into());
        }
        if child.try_wait().map_err(|e| e.to_string())?.is_some() {
            break;
        }
        if start.elapsed() > std::time::Duration::from_secs(10) {
            let _ = child.kill();
            let _ = child.wait();
            let _ = reader.join();
            return Err("ffmpeg timed out".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let (output, exceeded) = reader.join().map_err(|_| "ffmpeg reader failed")??;
    if exceeded {
        return Err("ffmpeg output exceeded limit".into());
    }
    if output.starts_with(&[0xff, 0xd8]) {
        Ok(output)
    } else {
        Err("ffmpeg produced no frame".into())
    }
}
fn encode(image: DynamicImage) -> image::ImageResult<Vec<u8>> {
    let thumb = image.thumbnail(320, 320);
    let mut bytes = Cursor::new(Vec::new());
    thumb.write_to(&mut bytes, ImageFormat::Jpeg)?;
    Ok(bytes.into_inner())
}
fn cache_key(r: &ThumbnailRequest) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in format!(
        "{}\0{}\0{}\0{}",
        r.source_id, r.relative_path, r.size, r.modified_unix
    )
    .bytes()
    {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    fn request(priority: u8, name: &str) -> ThumbnailRequest {
        ThumbnailRequest {
            source_id: "s".into(),
            relative_path: name.into(),
            size: 1,
            modified_unix: 1,
            path: name.into(),
            media_type: MediaType::Unknown,
            priority,
        }
    }

    #[test]
    fn priority_queue_prefers_visible_and_preserves_fifo() {
        let queue = JobQueue::new(3);
        let stop = Arc::new(AtomicBool::new(false));
        for (priority, name) in [(1, "background"), (9, "visible-a"), (9, "visible-b")] {
            queue
                .try_push(Job {
                    request: request(priority, name),
                    cache_dir: PathBuf::new(),
                    cancelled: Arc::clone(&stop),
                })
                .unwrap();
        }
        assert_eq!(queue.pop().unwrap().request.relative_path, "visible-a");
        assert_eq!(queue.pop().unwrap().request.relative_path, "visible-b");
        assert_eq!(queue.pop().unwrap().request.relative_path, "background");
        queue.close();
    }

    #[test]
    fn queue_rejects_without_blocking_when_full() {
        let queue = JobQueue::new(1);
        let stop = Arc::new(AtomicBool::new(false));
        queue
            .try_push(Job {
                request: request(0, "first"),
                cache_dir: PathBuf::new(),
                cancelled: Arc::clone(&stop),
            })
            .unwrap();
        assert!(matches!(
            queue.try_push(Job {
                request: request(0, "second"),
                cache_dir: PathBuf::new(),
                cancelled: stop
            }),
            Err(ThumbnailError::QueueFull)
        ));
        queue.close();
    }
    #[test]
    fn cache_key_changes_when_file_changes() {
        let a = ThumbnailRequest {
            source_id: "s".into(),
            relative_path: "a.jpg".into(),
            size: 1,
            modified_unix: 2,
            path: "a.jpg".into(),
            media_type: MediaType::Jpeg,
            priority: 0,
        };
        let mut b = a.clone();
        b.size = 3;
        assert_ne!(
            ThumbnailPipeline::cache_key(&a),
            ThumbnailPipeline::cache_key(&b)
        );
    }
    #[test]
    fn embedded_jpeg_fallback_picks_the_largest_stream() {
        let small = [0xff, 0xd8, 0xff, 0xe0, 1, 2, 3, 0xff, 0xd9];
        let large = [0xff, 0xd8, 0xff, 0xe0, 9, 9, 9, 9, 9, 9, 0xff, 0xd9];
        let mut data = vec![0u8; 12];
        data.extend_from_slice(&small);
        data.push(0);
        data.extend_from_slice(&large);
        assert_eq!(largest_embedded_jpeg(&data), Some(large.to_vec()));
        assert_eq!(largest_embedded_jpeg(&[0, 1, 2, 3]), None);
        // A stray start marker without an end marker is ignored, not panicking.
        assert_eq!(largest_embedded_jpeg(&[0xff, 0xd8, 0xff, 0, 0]), None);
    }

    #[test]
    fn dng_preview_thumbnail_falls_back_to_embedded_jpeg() {
        // A 4x4 JPEG in a buffer that has no EXIF thumbnail pointer, standing in
        // for a DNG whose preview is a JPEG-compressed TIFF strip.
        let mut jpeg = Cursor::new(Vec::new());
        DynamicImage::ImageRgb8(image::RgbImage::new(4, 4))
            .write_to(&mut jpeg, ImageFormat::Jpeg)
            .unwrap();
        let jpeg = jpeg.into_inner();
        let mut data = vec![0u8; 8];
        data.extend_from_slice(&jpeg);
        let preview = largest_embedded_jpeg(&data).unwrap();
        assert_eq!(preview, jpeg);
        assert!(image::load_from_memory(&preview).is_ok());
    }

    #[test]
    fn jpeg_is_generated_and_cached() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("a.jpg");
        let image = image::RgbImage::new(4, 4);
        image.save(&source).unwrap();
        let cache = dir.path().join("cache");
        let pipe = ThumbnailPipeline::new(&cache, 1, 2).unwrap();
        pipe.submit(ThumbnailRequest {
            source_id: "s".into(),
            relative_path: "a.jpg".into(),
            size: fs::metadata(&source).unwrap().len(),
            modified_unix: 1,
            path: source,
            media_type: MediaType::Jpeg,
            priority: 0,
        })
        .unwrap();
        for _ in 0..50 {
            if let Some(result) = pipe.try_recv() {
                assert_eq!(result.state, ThumbnailState::Ready);
                assert!(result.bytes.unwrap().len() > 10);
                return;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        panic!("thumbnail worker did not finish");
    }
}

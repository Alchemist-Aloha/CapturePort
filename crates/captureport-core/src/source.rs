use crate::{MediaItem, MediaLocator, ScanGeneration, SourceId};
use std::{
    fmt,
    io::Read,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceType {
    Filesystem,
    Camera,
    Fake,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceIdentity {
    pub id: SourceId,
    pub source_type: SourceType,
    pub stable_id: Option<String>,
    pub serial: Option<String>,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub volume_uuid: Option<String>,
    pub display_name: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct CancellationToken(Arc<AtomicBool>);
impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[derive(Clone, Debug)]
pub struct ScanContext {
    pub generation: ScanGeneration,
    cancellation: CancellationToken,
}
impl ScanContext {
    pub fn new(generation: ScanGeneration, cancellation: CancellationToken) -> Self {
        Self {
            generation,
            cancellation,
        }
    }
    pub fn cancellation(&self) -> &CancellationToken {
        &self.cancellation
    }
    pub fn is_cancelled(&self) -> bool {
        self.cancellation.is_cancelled()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SourceError {
    Cancelled,
    MissingItem(String),
    Io(String),
    Callback(String),
}
impl fmt::Display for SourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled => write!(f, "Scan cancelled"),
            Self::MissingItem(path) => write!(f, "Source item is no longer available: {path}"),
            Self::Io(detail) => write!(f, "Could not read source: {detail}"),
            Self::Callback(detail) => write!(f, "Could not deliver scan result: {detail}"),
        }
    }
}
impl std::error::Error for SourceError {}

#[derive(Clone, Copy, Debug)]
pub struct SourceReadProgress {
    pub bytes: u64,
    pub total: u64,
}

/// A streaming read can report camera-side work before delivering a chunk.
pub trait MediaStream: Read + Send {
    fn read_with_progress(
        &mut self,
        buffer: &mut [u8],
        cancellation: &CancellationToken,
        _progress: &mut dyn FnMut(SourceReadProgress),
    ) -> std::io::Result<usize> {
        if cancellation.is_cancelled() {
            return Err(std::io::Error::other("Transfer cancelled"));
        }
        self.read(buffer)
    }
}
impl MediaStream for Box<dyn Read + Send> {}

/// A source may be slow, non-seekable, or disconnected at any time.
pub trait MediaSource: Send + Sync {
    fn identity(&self) -> SourceIdentity;
    fn enumerate(
        &self,
        scan: &ScanContext,
        emit: &mut dyn FnMut(MediaItem) -> Result<(), SourceError>,
    ) -> Result<(), SourceError>;
    fn open_stream(&self, item: &MediaLocator) -> Result<Box<dyn Read + Send>, SourceError>;
    fn open_transfer(&self, item: &MediaLocator) -> Result<Box<dyn MediaStream>, SourceError> {
        Ok(Box::new(self.open_stream(item)?))
    }
    fn read_range(
        &self,
        item: &MediaLocator,
        offset: u64,
        length: usize,
    ) -> Result<Vec<u8>, SourceError>;

    /// Read an optional lightweight display preview for an item.
    ///
    /// Sources that do not provide previews keep the default implementation.
    /// Camera sources may return embedded preview bytes without opening the
    /// full-resolution stream.
    fn preview(&self, _item: &MediaLocator) -> Result<Option<Vec<u8>>, SourceError> {
        Ok(None)
    }
    fn preview_with_cancellation(
        &self,
        item: &MediaLocator,
        cancellation: &CancellationToken,
    ) -> Result<Option<Vec<u8>>, SourceError> {
        if cancellation.is_cancelled() {
            return Err(SourceError::Cancelled);
        }
        self.preview(item)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancelled_default_stream_and_preview_do_not_start_io() {
        let mut stream: Box<dyn Read + Send> = Box::new(std::io::Cursor::new(b"abc"));
        let cancellation = CancellationToken::new();
        cancellation.cancel();
        let mut buffer = [0; 3];
        assert!(stream
            .read_with_progress(&mut buffer, &cancellation, &mut |_| panic!(
                "unexpected progress"
            ))
            .is_err());
        assert_eq!(
            stream
                .read_with_progress(&mut buffer, &CancellationToken::new(), &mut |_| {})
                .unwrap(),
            3
        );
        assert_eq!(&buffer, b"abc");
        let source = crate::FakeSourceBuilder::new().build();
        assert_eq!(
            source.preview_with_cancellation(&MediaLocator("photo.JPG".into()), &cancellation),
            Err(SourceError::Cancelled)
        );
    }
}

use std::path::Path;

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SourceId(pub u64);
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MediaId(pub u64);
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BundleId(pub u64);
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ScanGeneration(pub u64);

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct MediaLocator(pub String);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MediaType {
    Raw,
    Jpeg,
    Heif,
    Png,
    Tiff,
    Video,
    Sidecar,
    Unknown,
}

pub fn classify_path(path: impl AsRef<Path>) -> MediaType {
    let extension = path
        .as_ref()
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match extension.as_str() {
        "arw" | "cr2" | "cr3" | "nef" | "nrw" | "orf" | "raf" | "rw2" | "dng" | "pef" | "srw" => {
            MediaType::Raw
        }
        "jpg" | "jpeg" | "jpe" => MediaType::Jpeg,
        "heif" | "heic" | "hif" => MediaType::Heif,
        "png" => MediaType::Png,
        "tif" | "tiff" => MediaType::Tiff,
        "mp4" | "mov" | "mts" | "m2ts" | "mkv" | "avi" | "3gp" | "3g2" | "webm" => MediaType::Video,
        "xmp" | "xml" | "aae" | "thm" | "json" => MediaType::Sidecar,
        _ => MediaType::Unknown,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MediaItem {
    pub id: MediaId,
    pub source_id: SourceId,
    pub source_path: String,
    pub source_name: String,
    pub locator: MediaLocator,
    pub size: u64,
    pub media_type: MediaType,
    pub metadata: MetadataState,
    pub import_status: ImportStatus,
    /// Set when `import_status` is `Imported` or `PossibleDuplicate` and the
    /// catalog could name the prior import behind that classification.
    pub prior_import: Option<PriorImport>,
    pub bundle_id: Option<BundleId>,
}

impl MediaItem {
    pub fn new(id: MediaId, source_id: SourceId, path: impl Into<String>, size: u64) -> Self {
        let source_path = path.into();
        let source_name = Path::new(&source_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&source_path)
            .to_owned();
        let media_type = classify_path(&source_path);
        Self {
            id,
            source_id,
            locator: MediaLocator(source_path.clone()),
            source_name,
            source_path,
            size,
            media_type,
            metadata: MetadataState::Pending,
            import_status: ImportStatus::New,
            prior_import: None,
            bundle_id: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ImportStatus {
    Checking,
    New,
    Imported,
    PossibleDuplicate,
    Unknown,
}

/// Names the prior import that a file matched, so the browser can say *why* a
/// frame looks already-present instead of only asserting that it does.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PriorImport {
    pub session_id: i64,
    pub imported_at: String,
    pub destination: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MetadataState {
    Pending,
    Ready(MediaMetadata),
    Failed(String),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MediaMetadata {
    pub capture_time: Option<String>,
    pub filesystem_time: Option<String>,
    pub timestamp_source: Option<TimestampSource>,
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub camera_serial: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub orientation: Option<Orientation>,
    pub duration_millis: Option<u64>,
    pub lens: Option<String>,
    /// Latitude and longitude scaled by 10,000,000, preserving Eq semantics.
    pub gps_e7: Option<(i32, i32)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimestampSource {
    ExifOriginal,
    QuickTime,
    Camera,
    Filesystem,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Orientation {
    Normal,
    Rotate90,
    Rotate180,
    Rotate270,
    MirrorHorizontal,
    MirrorVertical,
}

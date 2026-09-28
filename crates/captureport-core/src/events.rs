use crate::{MediaId, MediaItem, MediaMetadata, ScanGeneration, SourceId, SourceIdentity};

#[derive(Clone, Debug, PartialEq)]
pub struct ImportProgress {
    pub session_id: u64,
    pub completed: u64,
    pub total: u64,
    pub bytes_copied: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AppEvent {
    SourceDetected {
        generation: ScanGeneration,
        source: SourceIdentity,
    },
    SourceRemoved {
        generation: ScanGeneration,
        source_id: SourceId,
    },
    ScanStarted {
        generation: ScanGeneration,
        source_id: SourceId,
    },
    MediaDiscovered {
        generation: ScanGeneration,
        item: MediaItem,
    },
    MetadataReady {
        generation: ScanGeneration,
        media_id: MediaId,
        metadata: MediaMetadata,
    },
    MetadataFailed {
        generation: ScanGeneration,
        media_id: MediaId,
        error: String,
    },
    ThumbnailReady {
        generation: ScanGeneration,
        media_id: MediaId,
    },
    ImportStatusChanged {
        generation: ScanGeneration,
        media_id: MediaId,
        status: crate::ImportStatus,
        /// Present when the catalog could name the prior import behind the status.
        prior_import: Option<crate::PriorImport>,
    },
    ImportProgress {
        generation: ScanGeneration,
        progress: ImportProgress,
    },
    ImportCompleted {
        generation: ScanGeneration,
        session_id: u64,
    },
    ImportFailed {
        generation: ScanGeneration,
        session_id: u64,
        error: String,
    },
}

impl AppEvent {
    pub fn generation(&self) -> ScanGeneration {
        match self {
            Self::SourceDetected { generation, .. }
            | Self::SourceRemoved { generation, .. }
            | Self::ScanStarted { generation, .. }
            | Self::MediaDiscovered { generation, .. }
            | Self::MetadataReady { generation, .. }
            | Self::MetadataFailed { generation, .. }
            | Self::ThumbnailReady { generation, .. }
            | Self::ImportStatusChanged { generation, .. }
            | Self::ImportProgress { generation, .. }
            | Self::ImportCompleted { generation, .. }
            | Self::ImportFailed { generation, .. } => *generation,
        }
    }
}

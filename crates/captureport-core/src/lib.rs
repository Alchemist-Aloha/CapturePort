//! UI-independent CapturePort domain types and source contracts.

mod app_state;
mod events;
mod fake_source;
mod media;
mod source;

pub use app_state::{AppState, MediaFilter, MediaSort, SelectionSummary};
pub use events::{AppEvent, ImportProgress};
pub use fake_source::{FakeMediaSource, FakeSourceBuilder, FakeSourceScenario};
pub use media::{
    classify_path, BundleId, ImportStatus, MediaId, MediaItem, MediaLocator, MediaMetadata,
    MediaType, MetadataState, Orientation, ScanGeneration, SourceId, TimestampSource,
};
pub use source::{
    CancellationToken, MediaSource, ScanContext, SourceError, SourceIdentity, SourceType,
};

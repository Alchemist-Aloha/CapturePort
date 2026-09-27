//! UI-independent metadata and thumbnail adapters.
//!
//! The adapters deliberately return normalized types. No EXIF or image crate type
//! crosses this crate's public API.

mod metadata;
mod thumbnail;

pub use metadata::{MetadataError, MetadataService, NormalizedMetadata, TimestampSource};
pub use thumbnail::{
    ThumbnailError, ThumbnailFormat, ThumbnailPipeline, ThumbnailRequest, ThumbnailResult,
    ThumbnailState,
};

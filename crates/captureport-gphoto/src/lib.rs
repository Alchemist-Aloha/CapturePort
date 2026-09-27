//! Serialized libgphoto2 access and removable-device discovery.
//!
//! The C API is intentionally confined to `ffi` and the camera worker.  A
//! caller only deals with `GPhotoSource`, `MediaSource`, and plain discovery
//! records, so camera bindings can be replaced without changing the importer.

mod ffi;
mod worker;

pub use worker::{CameraDescriptor, GPhotoError, GPhotoSource, Preview, WorkerStats};

pub mod discovery;

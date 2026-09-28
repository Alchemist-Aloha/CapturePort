//! Persistent import history for CapturePort.
//!
//! The catalog is deliberately UI independent. [`CatalogHandle`] sends typed
//! requests to one worker thread, and that worker is the sole owner of the
//! SQLite connection.

mod database;
mod migrations;
mod repository;
mod worker;

pub use database::{CatalogError, CatalogPath};
pub use repository::{
    ImportRecord, ImportStatus, ImportedMatch, IncompleteSession, MediaIdentity, MediaRecord,
    MediaType, PresetRecord, SessionDetail, SessionRecord, SessionStatus, SourceIdentity,
    SourceRecord,
};
pub use worker::{
    CatalogCommand, CatalogHandle, CatalogResponse, ImportInput, ReconciledImportInput,
};

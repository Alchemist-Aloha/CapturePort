use std::path::{Path, PathBuf};

use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CatalogError {
    #[error("catalog worker is unavailable")]
    WorkerUnavailable,
    #[error("catalog request failed: {0}")]
    Request(String),
    #[error("catalog database error: {0}")]
    Database(String),
    #[error("catalog serialization error: {0}")]
    Serialization(String),
}

impl From<rusqlite::Error> for CatalogError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Database(error.to_string())
    }
}

/// Database location accepted by [`crate::CatalogHandle::open`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CatalogPath {
    File(PathBuf),
    Memory,
}

impl From<PathBuf> for CatalogPath {
    fn from(path: PathBuf) -> Self {
        Self::File(path)
    }
}
impl From<&Path> for CatalogPath {
    fn from(path: &Path) -> Self {
        Self::File(path.to_owned())
    }
}
impl From<&str> for CatalogPath {
    fn from(path: &str) -> Self {
        Self::File(PathBuf::from(path))
    }
}

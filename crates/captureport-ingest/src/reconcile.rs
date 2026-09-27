//! Explicit, read-only library reconciliation.

use crate::{Fingerprint, digest_reader, quick_path};
use captureport_core::{CancellationToken, MediaType, classify_path};
use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FakeFailureScenario {
    None,
    ReadFailure,
    Disconnected,
    CorruptFingerprint,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LibraryRecord {
    pub path: PathBuf,
    pub media_type: MediaType,
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub quick: Option<Fingerprint>,
    pub full: Option<Fingerprint>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ReconcileReport {
    pub records: Vec<LibraryRecord>,
    pub errors: Vec<String>,
    pub cancelled: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ReconcileOptions {
    pub calculate_full: bool,
}

pub fn scan_existing_library(
    roots: &[PathBuf],
    options: ReconcileOptions,
    cancellation: &CancellationToken,
) -> ReconcileReport {
    let mut report = ReconcileReport::default();
    let mut pending = roots.to_vec();
    pending.sort();
    while let Some(path) = pending.pop() {
        if cancellation.is_cancelled() {
            report.cancelled = true;
            break;
        }
        let metadata = match fs::symlink_metadata(&path) {
            Ok(value) => value,
            Err(error) => {
                report.errors.push(format!("{}: {error}", path.display()));
                continue;
            }
        };
        if metadata.is_dir() {
            match fs::read_dir(&path) {
                Ok(entries) => {
                    let mut children: Vec<_> =
                        entries.filter_map(Result::ok).map(|e| e.path()).collect();
                    children.sort();
                    pending.extend(children.into_iter().rev());
                }
                Err(error) => report.errors.push(format!("{}: {error}", path.display())),
            }
            continue;
        }
        if !metadata.is_file() || !is_media(&path) {
            continue;
        }
        let quick = match quick_path(&path) {
            Ok(value) => Some(value),
            Err(error) => {
                report.errors.push(format!("{}: {error}", path.display()));
                None
            }
        };
        let full = if options.calculate_full {
            match fs::File::open(&path).and_then(digest_reader) {
                Ok(digest) => Some(digest.full()),
                Err(error) => {
                    report.errors.push(format!("{}: {error}", path.display()));
                    None
                }
            }
        } else {
            None
        };
        report.records.push(LibraryRecord {
            media_type: classify_path(&path),
            path,
            size: metadata.len(),
            modified: metadata.modified().ok(),
            quick,
            full,
        });
    }
    report.records.sort_by(|a, b| a.path.cmp(&b.path));
    report
}

pub fn reconcile_library(roots: &[PathBuf], cancellation: &CancellationToken) -> ReconcileReport {
    scan_existing_library(roots, ReconcileOptions::default(), cancellation)
}

fn is_media(path: &Path) -> bool {
    !matches!(classify_path(path), MediaType::Unknown)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    #[test]
    fn scans_without_mutating_library() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("nested");
        fs::create_dir(&path).unwrap();
        let file = path.join("clip.JPG");
        fs::File::create(&file)
            .unwrap()
            .write_all(b"pixels")
            .unwrap();
        let before = fs::read(&file).unwrap();
        let report =
            reconcile_library(&[directory.path().to_path_buf()], &CancellationToken::new());
        assert_eq!(report.records.len(), 1);
        assert_eq!(fs::read(&file).unwrap(), before);
    }
    #[test]
    fn cancellation_stops_before_work() {
        let directory = tempfile::tempdir().unwrap();
        let token = CancellationToken::new();
        token.cancel();
        let report = reconcile_library(&[directory.path().to_path_buf()], &token);
        assert!(report.cancelled);
        assert!(report.records.is_empty());
    }
}

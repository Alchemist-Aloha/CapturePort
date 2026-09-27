use crate::{
    CopyDigest, ImportPlan, PlanStatus, PlannedCopy, PlannedImport, VerificationMode, verify_copy,
};
use captureport_core::{CancellationToken, MediaSource};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImportItemState {
    Planned,
    Queued,
    Copying,
    Verifying,
    Committing,
    Completed,
    Failed,
    Cancelled,
}

impl ImportItemState {
    fn advance(self, next: Self) -> Self {
        let valid = matches!(
            (self, next),
            (Self::Planned, Self::Queued)
                | (Self::Queued, Self::Copying)
                | (Self::Copying, Self::Verifying)
                | (Self::Verifying, Self::Committing)
                | (Self::Committing, Self::Completed)
                | (
                    Self::Queued | Self::Copying | Self::Verifying | Self::Committing,
                    Self::Failed
                )
                | (Self::Queued | Self::Copying, Self::Cancelled)
        );
        assert!(
            valid,
            "invalid import state transition: {self:?} -> {next:?}"
        );
        next
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ImportError {
    PlanBlocked,
    UnsupportedSource,
    SourceReadFailed(String),
    DestinationUnavailable(String),
    DestinationFull,
    PermissionDenied,
    Collision(String),
    VerificationFailed(String),
    DatabaseFailed(String),
    Cancelled,
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PlanBlocked => write!(f, "Resolve import preview conflicts before copying"),
            Self::UnsupportedSource => {
                write!(f, "The selected source no longer matches this import plan")
            }
            Self::SourceReadFailed(path) => write!(f, "Could not read source item {path}"),
            Self::DestinationUnavailable(path) => write!(f, "Destination is unavailable: {path}"),
            Self::DestinationFull => write!(f, "Destination has insufficient space"),
            Self::PermissionDenied => write!(f, "Destination permission was denied"),
            Self::Collision(path) => write!(f, "Destination already exists: {path}"),
            Self::VerificationFailed(path) => write!(f, "Verification failed for {path}"),
            Self::DatabaseFailed(detail) => {
                write!(f, "Copied file could not be recorded: {detail}")
            }
            Self::Cancelled => write!(f, "Import cancelled"),
        }
    }
}
impl std::error::Error for ImportError {}

#[derive(Clone, Debug)]
pub struct ImportProgress {
    pub media_id: captureport_core::MediaId,
    pub current_file_bytes: u64,
    pub current_file_total: u64,
    pub overall_bytes: u64,
    pub overall_total: u64,
    pub files_completed: usize,
    pub files_total: usize,
    pub bytes_per_second: f64,
}

#[derive(Clone, Debug)]
pub struct CopyResult {
    pub destination: std::path::PathBuf,
    pub state: ImportItemState,
    pub error: Option<ImportError>,
}

#[derive(Clone, Debug)]
pub struct ItemResult {
    pub media_id: captureport_core::MediaId,
    pub copies: Vec<CopyResult>,
}

#[derive(Clone, Debug, Default)]
pub struct ImportResult {
    pub items: Vec<ItemResult>,
    pub cancelled: bool,
}

pub trait ImportRecorder {
    /// Called only after a verified final destination has been committed.
    fn record_success(
        &mut self,
        item: &PlannedImport,
        copy: &PlannedCopy,
        digest: &CopyDigest,
        verification: VerificationMode,
    ) -> Result<(), String>;
}

pub struct NoopRecorder;
impl ImportRecorder for NoopRecorder {
    fn record_success(
        &mut self,
        _: &PlannedImport,
        _: &PlannedCopy,
        _: &CopyDigest,
        _: VerificationMode,
    ) -> Result<(), String> {
        Ok(())
    }
}

pub struct ImportEngine;

impl ImportEngine {
    pub fn execute(
        source: &dyn MediaSource,
        plan: &ImportPlan,
        cancellation: &CancellationToken,
        recorder: &mut dyn ImportRecorder,
        mut progress: impl FnMut(ImportProgress),
    ) -> Result<ImportResult, ImportError> {
        if source.identity().id != plan.source.id {
            return Err(ImportError::UnsupportedSource);
        }
        if plan
            .items
            .iter()
            .any(|item| !item.status.can_execute() && item.status != PlanStatus::Skipped)
        {
            return Err(ImportError::PlanBlocked);
        }
        let overall_total = plan
            .items
            .iter()
            .flat_map(|item| &item.copies)
            .filter(|copy| copy.status == PlanStatus::Ready)
            .map(|copy| copy.expected_size)
            .sum();
        let files_total = plan
            .items
            .iter()
            .flat_map(|item| &item.copies)
            .filter(|copy| copy.status == PlanStatus::Ready)
            .count();
        let started = Instant::now();
        let mut last_progress = Instant::now() - Duration::from_secs(1);
        let mut overall_bytes = 0u64;
        let mut files_completed = 0usize;
        let mut result = ImportResult::default();

        for item in &plan.items {
            if cancellation.is_cancelled() {
                result.cancelled = true;
                break;
            }
            let mut item_result = ItemResult {
                media_id: item.media_id,
                copies: Vec::new(),
            };
            for copy in &item.copies {
                if cancellation.is_cancelled() {
                    result.cancelled = true;
                    break;
                }
                if copy.status == PlanStatus::ExistingIdentical {
                    let check = source
                        .open_stream(&item.source)
                        .map_err(|_| ImportError::SourceReadFailed(item.source_name.clone()))
                        .and_then(|stream| {
                            crate::digest_reader(stream).map_err(|_| {
                                ImportError::SourceReadFailed(item.source_name.clone())
                            })
                        })
                        .and_then(|digest| {
                            let mode = if plan.verification == VerificationMode::Fast {
                                VerificationMode::Standard
                            } else {
                                plan.verification
                            };
                            match verify_copy(mode, &digest, &copy.final_destination) {
                                Ok(true) => recorder
                                    .record_success(item, copy, &digest, mode)
                                    .map_err(ImportError::DatabaseFailed),
                                Ok(false) => Err(ImportError::VerificationFailed(
                                    copy.final_destination.display().to_string(),
                                )),
                                Err(error) => {
                                    Err(ImportError::DestinationUnavailable(error.to_string()))
                                }
                            }
                        });
                    item_result.copies.push(CopyResult {
                        destination: copy.final_destination.clone(),
                        state: if check.is_ok() {
                            ImportItemState::Completed
                        } else {
                            ImportItemState::Failed
                        },
                        error: check.err(),
                    });
                    continue;
                }
                if copy.status != PlanStatus::Ready {
                    let error = match copy.status {
                        PlanStatus::Skipped => None,
                        PlanStatus::DestinationCollision => Some(ImportError::Collision(
                            copy.final_destination.display().to_string(),
                        )),
                        PlanStatus::DestinationUnavailable => {
                            Some(ImportError::DestinationUnavailable(
                                copy.destination_root.display().to_string(),
                            ))
                        }
                        PlanStatus::InsufficientDiskSpace => Some(ImportError::DestinationFull),
                        PlanStatus::InvalidPath => Some(ImportError::DestinationUnavailable(
                            "Invalid backup path".into(),
                        )),
                        PlanStatus::UnsupportedSource => Some(ImportError::UnsupportedSource),
                        PlanStatus::Ready | PlanStatus::ExistingIdentical => None,
                    };
                    item_result.copies.push(CopyResult {
                        destination: copy.final_destination.clone(),
                        state: if error.is_some() {
                            ImportItemState::Failed
                        } else {
                            ImportItemState::Planned
                        },
                        error,
                    });
                    continue;
                }
                let operation = Self::copy_one(
                    source,
                    item,
                    copy,
                    plan.verification,
                    cancellation,
                    &mut |current_bytes| {
                        if last_progress.elapsed() >= Duration::from_millis(100)
                            || current_bytes == copy.expected_size
                        {
                            let elapsed = started.elapsed().as_secs_f64().max(0.001);
                            progress(ImportProgress {
                                media_id: item.media_id,
                                current_file_bytes: current_bytes,
                                current_file_total: copy.expected_size,
                                overall_bytes: overall_bytes.saturating_add(current_bytes),
                                overall_total,
                                files_completed,
                                files_total,
                                bytes_per_second: (overall_bytes.saturating_add(current_bytes))
                                    as f64
                                    / elapsed,
                            });
                            last_progress = Instant::now();
                        }
                    },
                );
                match operation {
                    Ok(digest) => {
                        overall_bytes = overall_bytes.saturating_add(digest.size());
                        files_completed += 1;
                        let database_result = recorder
                            .record_success(item, copy, &digest, plan.verification)
                            .map_err(ImportError::DatabaseFailed);
                        item_result.copies.push(CopyResult {
                            destination: copy.final_destination.clone(),
                            state: if database_result.is_ok() {
                                ImportItemState::Completed
                            } else {
                                ImportItemState::Failed
                            },
                            error: database_result.err(),
                        });
                    }
                    Err(error) => {
                        let cancelled = error == ImportError::Cancelled;
                        item_result.copies.push(CopyResult {
                            destination: copy.final_destination.clone(),
                            state: if cancelled {
                                ImportItemState::Cancelled
                            } else {
                                ImportItemState::Failed
                            },
                            error: Some(error),
                        });
                        if cancelled {
                            result.cancelled = true;
                            break;
                        }
                        if copy.required {
                            break;
                        }
                    }
                }
            }
            result.items.push(item_result);
            if result.cancelled {
                break;
            }
        }
        Ok(result)
    }

    fn copy_one(
        source: &dyn MediaSource,
        item: &PlannedImport,
        copy: &PlannedCopy,
        verification: VerificationMode,
        cancellation: &CancellationToken,
        progress: &mut dyn FnMut(u64),
    ) -> Result<CopyDigest, ImportError> {
        let mut state = ImportItemState::Planned.advance(ImportItemState::Queued);
        if cancellation.is_cancelled() {
            return Err(ImportError::Cancelled);
        }
        let parent = copy.final_destination.parent().ok_or_else(|| {
            ImportError::DestinationUnavailable(copy.final_destination.display().to_string())
        })?;
        fs::create_dir_all(parent).map_err(map_io)?;
        let canonical_root = fs::canonicalize(&copy.destination_root).map_err(map_io)?;
        let canonical_parent = fs::canonicalize(parent).map_err(map_io)?;
        if !canonical_parent.starts_with(&canonical_root) {
            return Err(ImportError::DestinationUnavailable(
                copy.final_destination.display().to_string(),
            ));
        }
        if copy.final_destination.exists() {
            return Err(ImportError::Collision(
                copy.final_destination.display().to_string(),
            ));
        }
        let mut temporary = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&copy.temporary_destination)
            .map_err(map_io)?;
        let operation = (|| -> Result<CopyDigest, ImportError> {
            state = state.advance(ImportItemState::Copying);
            let mut reader = source
                .open_stream(&item.source)
                .map_err(|_| ImportError::SourceReadFailed(item.source_name.clone()))?;
            let mut digest = CopyDigest::new();
            let mut buffer = [0u8; 128 * 1024];
            loop {
                if cancellation.is_cancelled() {
                    return Err(ImportError::Cancelled);
                }
                let amount = reader
                    .read(&mut buffer)
                    .map_err(|_| ImportError::SourceReadFailed(item.source_name.clone()))?;
                if amount == 0 {
                    break;
                }
                temporary.write_all(&buffer[..amount]).map_err(map_io)?;
                digest.update(&buffer[..amount]);
                progress(digest.size());
            }
            if digest.size() != item.expected_size {
                return Err(ImportError::SourceReadFailed(item.source_name.clone()));
            }
            temporary.flush().map_err(map_io)?;
            temporary.sync_all().map_err(map_io)?;
            drop(temporary);
            state = state.advance(ImportItemState::Verifying);
            if !verify_copy(verification, &digest, &copy.temporary_destination).map_err(map_io)? {
                return Err(ImportError::VerificationFailed(item.source_name.clone()));
            }
            if cancellation.is_cancelled() {
                return Err(ImportError::Cancelled);
            }
            state = state.advance(ImportItemState::Committing);
            // hard_link is atomic and refuses to replace a file that appeared after planning.
            fs::hard_link(&copy.temporary_destination, &copy.final_destination).map_err(
                |error| {
                    if error.kind() == io::ErrorKind::AlreadyExists {
                        ImportError::Collision(copy.final_destination.display().to_string())
                    } else {
                        map_io(error)
                    }
                },
            )?;
            let _ = fs::remove_file(&copy.temporary_destination);
            File::open(parent)
                .and_then(|directory| directory.sync_all())
                .map_err(map_io)?;
            state = state.advance(ImportItemState::Completed);
            debug_assert_eq!(state, ImportItemState::Completed);
            Ok(digest)
        })();
        if operation.is_err() {
            let _ = fs::remove_file(&copy.temporary_destination);
        }
        operation
    }
}

fn map_io(error: io::Error) -> ImportError {
    match error.kind() {
        io::ErrorKind::PermissionDenied => ImportError::PermissionDenied,
        io::ErrorKind::StorageFull => ImportError::DestinationFull,
        _ => ImportError::DestinationUnavailable(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ImportPlanner, ImportPreset, PlanInput};
    use captureport_core::{MediaItem, ScanContext, ScanGeneration};
    use captureport_media::FilesystemSource;
    use chrono::{FixedOffset, TimeZone};
    use std::path::Path;

    fn prepared(source_root: &Path, destination_root: &Path) -> (FilesystemSource, ImportPlan) {
        let source = FilesystemSource::new(source_root).unwrap();
        let mut items = Vec::<MediaItem>::new();
        source
            .enumerate(
                &ScanContext::new(ScanGeneration(1), CancellationToken::new()),
                &mut |item| {
                    items.push(item);
                    Ok(())
                },
            )
            .unwrap();
        let time = FixedOffset::east_opt(0)
            .unwrap()
            .with_ymd_and_hms(2026, 9, 27, 14, 30, 0)
            .unwrap();
        let mut preset = ImportPreset::everyday(destination_root);
        preset.photo.root = destination_root.to_path_buf();
        preset.photo.folder_template.clear();
        let plan = ImportPlanner::build(
            &source,
            items
                .into_iter()
                .map(|item| PlanInput {
                    item,
                    capture_time: time,
                })
                .collect(),
            &preset,
        );
        (source, plan)
    }

    #[test]
    fn state_machine_rejects_invalid_transitions() {
        assert_eq!(
            ImportItemState::Planned.advance(ImportItemState::Queued),
            ImportItemState::Queued
        );
    }

    #[test]
    fn copies_verifies_commits_and_records_after_final_exists() {
        struct Recorder {
            calls: usize,
        }
        impl ImportRecorder for Recorder {
            fn record_success(
                &mut self,
                _: &PlannedImport,
                copy: &PlannedCopy,
                _: &CopyDigest,
                _: VerificationMode,
            ) -> Result<(), String> {
                assert_eq!(fs::read(&copy.final_destination).unwrap(), b"original");
                assert!(!copy.temporary_destination.exists());
                self.calls += 1;
                Ok(())
            }
        }
        let source_dir = tempfile::tempdir().unwrap();
        let destination_dir = tempfile::tempdir().unwrap();
        fs::write(source_dir.path().join("DSC0001.JPG"), b"original").unwrap();
        let (source, plan) = prepared(source_dir.path(), destination_dir.path());
        assert_eq!(plan.items[0].status, PlanStatus::Ready);
        let mut recorder = Recorder { calls: 0 };
        let result = ImportEngine::execute(
            &source,
            &plan,
            &CancellationToken::new(),
            &mut recorder,
            |_| {},
        )
        .unwrap();
        assert_eq!(result.items[0].copies[0].state, ImportItemState::Completed);
        assert_eq!(recorder.calls, 1);
        assert_eq!(
            fs::read(&plan.items[0].copies[0].final_destination).unwrap(),
            b"original"
        );
        assert_eq!(
            fs::read(source_dir.path().join("DSC0001.JPG")).unwrap(),
            b"original"
        );
    }

    #[test]
    fn cancellation_leaves_no_incomplete_final_file() {
        let source_dir = tempfile::tempdir().unwrap();
        let destination_dir = tempfile::tempdir().unwrap();
        fs::write(
            source_dir.path().join("large.JPG"),
            vec![7; 3 * 1024 * 1024],
        )
        .unwrap();
        let (source, plan) = prepared(source_dir.path(), destination_dir.path());
        let cancellation = CancellationToken::new();
        let copy = &plan.items[0].copies[0];
        let result =
            ImportEngine::execute(&source, &plan, &cancellation, &mut NoopRecorder, |_| {
                cancellation.cancel()
            })
            .unwrap();
        assert!(result.cancelled);
        assert!(!copy.final_destination.exists());
        assert!(!copy.temporary_destination.exists());
        assert!(source_dir.path().join("large.JPG").exists());
    }

    #[test]
    fn plan_rejects_an_existing_different_destination() {
        let source_dir = tempfile::tempdir().unwrap();
        let destination_dir = tempfile::tempdir().unwrap();
        fs::write(source_dir.path().join("same.JPG"), b"new").unwrap();
        fs::write(destination_dir.path().join("same.JPG"), b"old").unwrap();
        let (source, plan) = prepared(source_dir.path(), destination_dir.path());
        assert_eq!(plan.items[0].status, PlanStatus::DestinationCollision);
        assert_eq!(
            ImportEngine::execute(
                &source,
                &plan,
                &CancellationToken::new(),
                &mut NoopRecorder,
                |_| {}
            )
            .unwrap_err(),
            ImportError::PlanBlocked
        );
        assert_eq!(
            fs::read(destination_dir.path().join("same.JPG")).unwrap(),
            b"old"
        );
    }
    #[test]
    fn existing_identical_is_reverified_and_recorded_without_overwrite() {
        let source_dir = tempfile::tempdir().unwrap();
        let destination_dir = tempfile::tempdir().unwrap();
        fs::write(source_dir.path().join("same.JPG"), b"original").unwrap();
        fs::write(destination_dir.path().join("same.JPG"), b"original").unwrap();
        let (source, plan) = prepared(source_dir.path(), destination_dir.path());
        assert_eq!(plan.items[0].status, PlanStatus::ExistingIdentical);
        let result = ImportEngine::execute(
            &source,
            &plan,
            &CancellationToken::new(),
            &mut NoopRecorder,
            |_| {},
        )
        .unwrap();
        assert_eq!(result.items[0].copies[0].state, ImportItemState::Completed);
        fs::write(source_dir.path().join("same.JPG"), b"altered!").unwrap();
        let changed = ImportEngine::execute(
            &source,
            &plan,
            &CancellationToken::new(),
            &mut NoopRecorder,
            |_| {},
        )
        .unwrap();
        assert_eq!(changed.items[0].copies[0].state, ImportItemState::Failed);
        assert_eq!(
            fs::read(destination_dir.path().join("same.JPG")).unwrap(),
            b"original"
        );
    }
    #[test]
    fn unavailable_optional_backup_keeps_primary_success_and_reports_failure() {
        let source_dir = tempfile::tempdir().unwrap();
        let destination_dir = tempfile::tempdir().unwrap();
        fs::write(source_dir.path().join("photo.JPG"), b"photo").unwrap();
        let source = FilesystemSource::new(source_dir.path()).unwrap();
        let mut items = Vec::new();
        source
            .enumerate(
                &ScanContext::new(ScanGeneration(1), CancellationToken::new()),
                &mut |item| {
                    items.push(item);
                    Ok(())
                },
            )
            .unwrap();
        let mut preset = ImportPreset::everyday(destination_dir.path());
        preset.photo.root = destination_dir.path().to_path_buf();
        preset.photo.folder_template.clear();
        let missing = destination_dir.path().join("missing-backup-root");
        preset.backup = Some(crate::BackupRule {
            photo: crate::DestinationRule {
                root: missing.clone(),
                folder_template: String::new(),
            },
            video: crate::DestinationRule {
                root: missing,
                folder_template: String::new(),
            },
            required: false,
        });
        let time = FixedOffset::east_opt(0)
            .unwrap()
            .with_ymd_and_hms(2026, 9, 27, 0, 0, 0)
            .unwrap();
        let plan = ImportPlanner::build(
            &source,
            items
                .into_iter()
                .map(|item| PlanInput {
                    item,
                    capture_time: time,
                })
                .collect(),
            &preset,
        );
        assert_eq!(plan.items[0].status, PlanStatus::Ready);
        assert_eq!(
            plan.items[0].copies[1].status,
            PlanStatus::DestinationUnavailable
        );
        let result = ImportEngine::execute(
            &source,
            &plan,
            &CancellationToken::new(),
            &mut NoopRecorder,
            |_| {},
        )
        .unwrap();
        assert_eq!(result.items[0].copies[0].state, ImportItemState::Completed);
        assert_eq!(result.items[0].copies[1].state, ImportItemState::Failed);
        assert_eq!(
            fs::read(&plan.items[0].copies[0].final_destination).unwrap(),
            b"photo"
        );
        assert!(source_dir.path().join("photo.JPG").exists());
    }
}

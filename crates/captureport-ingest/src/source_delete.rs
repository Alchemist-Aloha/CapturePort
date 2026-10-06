//! Explicit, opt-in deletion of source media after verified import copies.
//!
//! Source deletion is deliberately kept outside [`crate::ImportEngine`].  A
//! caller must opt in for each operation, and the source adapter must
//! implement [`SourceRemover`] itself.  This keeps the common import path
//! read-only with respect to cameras and removable media.

use crate::{ImportItemState, ImportResult, PlannedImport};
use captureport_core::{MediaId, MediaLocator};
use std::fmt;

/// An adapter capable of removing one source item.
///
/// Filesystem and camera adapters can implement this with their native
/// deletion API.  The ingest crate never guesses how a source should be
/// mutated and never recursively removes a directory.
pub trait SourceRemover {
    type Error: fmt::Display;

    fn remove(&mut self, source: &MediaLocator) -> Result<(), Self::Error>;
}

/// Conditions that must be satisfied before source deletion is attempted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeletionOptions {
    /// A separate affirmative action is required for every deletion call.
    pub confirmed: bool,
    /// Require every planned copy, including optional backups, to have been
    /// completed and verified in this import result.
    pub require_all_destinations: bool,
}

impl Default for DeletionOptions {
    fn default() -> Self {
        Self {
            confirmed: false,
            require_all_destinations: true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeletionRefusal {
    NotConfirmed,
    MissingImportResult,
    RequiredCopyNotVerified,
    BackupCopyNotVerified,
}

impl fmt::Display for DeletionRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotConfirmed => write!(f, "Source deletion was not explicitly confirmed"),
            Self::MissingImportResult => write!(f, "The import result has no record for this item"),
            Self::RequiredCopyNotVerified => {
                write!(f, "A required destination was not copied and verified")
            }
            Self::BackupCopyNotVerified => {
                write!(f, "A backup destination was not copied and verified")
            }
        }
    }
}

#[derive(Debug)]
pub struct DeletionFailure {
    pub media_id: MediaId,
    pub reason: String,
}

#[derive(Debug, Default)]
pub struct DeletionReport {
    pub deleted: Vec<MediaId>,
    pub refused: Vec<DeletionFailure>,
    pub failed: Vec<DeletionFailure>,
}

/// Check whether all required copies for one item reached the verified
/// committed state.  `ImportItemState::Completed` is only produced after the
/// importer has verified and committed the destination file.
pub fn deletion_refusal(
    item: &PlannedImport,
    result: &ImportResult,
    options: DeletionOptions,
) -> Result<(), DeletionRefusal> {
    if !options.confirmed {
        return Err(DeletionRefusal::NotConfirmed);
    }
    let item_result = result
        .items
        .iter()
        .find(|candidate| candidate.media_id == item.media_id)
        .ok_or(DeletionRefusal::MissingImportResult)?;

    for planned in &item.copies {
        let Some(copy) = item_result
            .copies
            .iter()
            .find(|candidate| candidate.destination == planned.final_destination)
        else {
            if planned.required {
                return Err(DeletionRefusal::RequiredCopyNotVerified);
            }
            if options.require_all_destinations {
                return Err(DeletionRefusal::BackupCopyNotVerified);
            }
            continue;
        };

        if copy.state != ImportItemState::Completed {
            if planned.required {
                return Err(DeletionRefusal::RequiredCopyNotVerified);
            }
            if options.require_all_destinations {
                return Err(DeletionRefusal::BackupCopyNotVerified);
            }
        }
    }
    Ok(())
}

/// Delete explicitly selected source items whose copies are verified.
///
/// The function processes items independently so a failed camera operation
/// does not hide successful deletions for other selected items.  It does not
/// retry or delete a source item when the eligibility check fails.
pub fn delete_verified_sources<R: SourceRemover>(
    remover: &mut R,
    items: &[PlannedImport],
    result: &ImportResult,
    options: DeletionOptions,
) -> DeletionReport {
    let mut report = DeletionReport::default();
    for item in items {
        if let Err(reason) = deletion_refusal(item, result, options) {
            report.refused.push(DeletionFailure {
                media_id: item.media_id,
                reason: reason.to_string(),
            });
            continue;
        }
        match remover.remove(&item.source) {
            Ok(()) => report.deleted.push(item.media_id),
            Err(error) => report.failed.push(DeletionFailure {
                media_id: item.media_id,
                reason: error.to_string(),
            }),
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CopyResult, ImportItemState, PlanStatus, PlannedCopy, VerificationMode};
    use captureport_core::{MediaId, MediaLocator, SourceId, SourceIdentity, SourceType};
    use std::path::PathBuf;

    #[derive(Default)]
    struct FakeRemover {
        removed: Vec<MediaLocator>,
        fail: bool,
    }

    impl SourceRemover for FakeRemover {
        type Error = &'static str;

        fn remove(&mut self, source: &MediaLocator) -> Result<(), Self::Error> {
            if self.fail {
                return Err("camera disconnected");
            }
            self.removed.push(source.clone());
            Ok(())
        }
    }

    fn item(media_id: u64, copies: Vec<(bool, &str)>) -> PlannedImport {
        PlannedImport {
            media_id: MediaId(media_id),
            source: MediaLocator(format!("DCIM/IMG{media_id:04}.JPG")),
            source_name: format!("IMG{media_id:04}.JPG"),
            expected_size: 10,
            effective_time: chrono::DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z").unwrap(),
            sequence: 1,
            session: 1,
            copies: copies
                .into_iter()
                .map(|(required, path)| PlannedCopy {
                    destination_identity: None,
                    destination_root: PathBuf::from("/library"),
                    final_destination: PathBuf::from(path),
                    temporary_destination: PathBuf::from("/library/.tmp"),
                    expected_size: 10,
                    required,
                    status: PlanStatus::Ready,
                })
                .collect(),
            status: PlanStatus::Ready,
        }
    }

    fn result(media_id: u64, destinations: &[(&str, ImportItemState)]) -> ImportResult {
        ImportResult {
            items: vec![crate::ItemResult {
                media_id: MediaId(media_id),
                copies: destinations
                    .iter()
                    .map(|(path, state)| CopyResult {
                        destination: PathBuf::from(path),
                        state: *state,
                        error: None,
                    })
                    .collect(),
            }],
            cancelled: false,
        }
    }

    fn options() -> DeletionOptions {
        DeletionOptions {
            confirmed: true,
            require_all_destinations: true,
        }
    }

    #[test]
    fn only_verified_required_and_backup_copies_can_be_deleted() {
        let selected = item(
            1,
            vec![(true, "/library/one.jpg"), (false, "/backup/one.jpg")],
        );
        let imported = result(
            1,
            &[
                ("/library/one.jpg", ImportItemState::Completed),
                ("/backup/one.jpg", ImportItemState::Completed),
            ],
        );
        let mut remover = FakeRemover::default();
        let report = delete_verified_sources(&mut remover, &[selected], &imported, options());
        assert_eq!(report.deleted, vec![MediaId(1)]);
        assert!(report.refused.is_empty());
        assert_eq!(remover.removed.len(), 1);
    }

    #[test]
    fn failed_required_copy_is_never_deleted() {
        let selected = item(1, vec![(true, "/library/one.jpg")]);
        let imported = result(1, &[("/library/one.jpg", ImportItemState::Failed)]);
        let mut remover = FakeRemover::default();
        let report = delete_verified_sources(&mut remover, &[selected], &imported, options());
        assert!(report.deleted.is_empty());
        assert_eq!(report.refused[0].media_id, MediaId(1));
        assert!(remover.removed.is_empty());
    }

    #[test]
    fn optional_backup_can_be_ignored_when_policy_allows_it() {
        let selected = item(
            1,
            vec![(true, "/library/one.jpg"), (false, "/backup/one.jpg")],
        );
        let imported = result(1, &[("/library/one.jpg", ImportItemState::Completed)]);
        let mut remover = FakeRemover::default();
        let report = delete_verified_sources(
            &mut remover,
            &[selected],
            &imported,
            DeletionOptions {
                confirmed: true,
                require_all_destinations: false,
            },
        );
        assert_eq!(report.deleted, vec![MediaId(1)]);
    }

    #[test]
    fn unconfirmed_operation_is_read_only() {
        let selected = item(1, vec![(true, "/library/one.jpg")]);
        let imported = result(1, &[("/library/one.jpg", ImportItemState::Completed)]);
        let mut remover = FakeRemover::default();
        let report = delete_verified_sources(
            &mut remover,
            &[selected],
            &imported,
            DeletionOptions::default(),
        );
        assert!(report.deleted.is_empty());
        assert_eq!(
            report.refused[0].reason,
            "Source deletion was not explicitly confirmed"
        );
        assert!(remover.removed.is_empty());
    }

    #[test]
    fn adapter_errors_are_reported_without_retrying() {
        let selected = item(1, vec![(true, "/library/one.jpg")]);
        let imported = result(1, &[("/library/one.jpg", ImportItemState::Completed)]);
        let mut remover = FakeRemover {
            fail: true,
            ..FakeRemover::default()
        };
        let report = delete_verified_sources(&mut remover, &[selected], &imported, options());
        assert!(report.deleted.is_empty());
        assert_eq!(report.failed[0].reason, "camera disconnected");
        assert!(remover.removed.is_empty());
    }

    #[test]
    fn identity_types_are_available_to_source_adapters() {
        let identity = SourceIdentity {
            id: SourceId(1),
            source_type: SourceType::Camera,
            stable_id: None,
            serial: None,
            manufacturer: None,
            model: None,
            volume_uuid: None,
            display_name: None,
        };
        assert_eq!(identity.source_type, SourceType::Camera);
        let _ = VerificationMode::Standard;
    }
}

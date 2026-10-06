use crate::*;

pub(crate) enum WorkMessage {
    Event(Box<AppEvent>),
    Plan(u64, ImportPlan),
    History(Vec<HistoryEntry>),
    Recovery(Vec<IncompleteSession>, Vec<captureport_ingest::PartialFile>),
    PartialCleaned(PathBuf),
    ImportResult(ScanGeneration, captureport_ingest::ImportResult),
    CacheCleared(usize),
    SourceAlias(i64, Option<String>),
    Presets(Result<(Vec<captureport_catalog::PresetRecord>, Option<i64>, String), String>),
    MarkedImported {
        generation: ScanGeneration,
        media_ids: Vec<MediaId>,
        result: Result<(), String>,
    },
    ImportDone(ScanGeneration, Result<String, String>),
    ReconcileDone(Result<String, String>),
    Done(Result<String, String>),
    Mounted(Result<String, String>),
}
pub(crate) struct CameraPreviewJob {
    pub(crate) generation: u64,
    pub(crate) id: MediaId,
    pub(crate) source: Arc<dyn MediaSource>,
    pub(crate) locator: MediaLocator,
    pub(crate) cache_path: PathBuf,
    pub(crate) cancellation: CancellationToken,
}
pub(crate) struct CameraPreviewResult {
    pub(crate) generation: u64,
    pub(crate) id: MediaId,
    pub(crate) cache_path: Option<PathBuf>,
}
pub(crate) struct VerifiedFilesystemRemover {
    pub(crate) root: PathBuf,
    pub(crate) copies: HashMap<String, Vec<PathBuf>>,
    pub(crate) sizes: HashMap<String, u64>,
}
impl VerifiedFilesystemRemover {
    pub(crate) fn new(root: PathBuf, plan: &ImportPlan) -> Self {
        let mut copies = HashMap::new();
        let mut sizes = HashMap::new();
        for item in &plan.items {
            copies.insert(
                item.source.0.clone(),
                item.copies
                    .iter()
                    .map(|copy| copy.final_destination.clone())
                    .collect(),
            );
            sizes.insert(item.source.0.clone(), item.expected_size);
        }
        Self {
            root,
            copies,
            sizes,
        }
    }
}
impl captureport_ingest::SourceRemover for VerifiedFilesystemRemover {
    type Error = String;
    fn remove(&mut self, source: &captureport_core::MediaLocator) -> Result<(), Self::Error> {
        let relative = std::path::Path::new(&source.0);
        if relative
            .components()
            .any(|part| !matches!(part, std::path::Component::Normal(_)))
        {
            return Err("Source locator is not a safe relative path".into());
        }
        let original = self.root.join(&source.0);
        if !std::fs::symlink_metadata(&original)
            .map_err(|e| e.to_string())?
            .file_type()
            .is_file()
        {
            return Err("Source path is not a regular file".into());
        }
        let root = std::fs::canonicalize(&self.root).map_err(|e| e.to_string())?;
        let candidate = std::fs::canonicalize(&original).map_err(|e| e.to_string())?;
        if candidate != root.join(relative) {
            return Err("Source path changed after import".into());
        }
        if !candidate.starts_with(&root) {
            return Err("Source path escaped its selected root".into());
        }
        let metadata = std::fs::symlink_metadata(&candidate).map_err(|e| e.to_string())?;
        if !metadata.file_type().is_file() {
            return Err("Source is no longer a regular file".into());
        }
        let expected = self
            .sizes
            .get(&source.0)
            .ok_or("Source was not in the import plan")?;
        if metadata.len() != *expected {
            return Err("Source size changed after import".into());
        }
        let source_hash = captureport_ingest::digest_reader(
            std::fs::File::open(&candidate).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?
        .full()
        .hex;
        let copies = self
            .copies
            .get(&source.0)
            .ok_or("No verified destinations were planned")?;
        for destination in copies {
            if *destination == candidate {
                return Err("Source and destination resolve to the same file".into());
            }
            if !std::fs::symlink_metadata(destination)
                .map_err(|e| e.to_string())?
                .file_type()
                .is_file()
            {
                return Err(format!(
                    "Destination is no longer a regular file: {}",
                    destination.display()
                ));
            }
            if std::fs::canonicalize(destination).map_err(|e| e.to_string())? == candidate {
                return Err("Destination refers to the original file".into());
            }
            let destination_hash = captureport_ingest::digest_reader(
                std::fs::File::open(destination).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?
            .full()
            .hex;
            if destination_hash != source_hash {
                return Err(format!(
                    "Destination changed after import: {}",
                    destination.display()
                ));
            }
        }
        std::fs::remove_file(candidate).map_err(|e| e.to_string())
    }
}
pub(crate) struct Recorder {
    pub(crate) catalog: CatalogHandle,
    pub(crate) session_id: i64,
    pub(crate) media_ids: HashMap<MediaId, i64>,
}
impl ImportRecorder for Recorder {
    fn record_success(
        &mut self,
        item: &PlannedImport,
        copy: &PlannedCopy,
        digest: &CopyDigest,
        verification: VerificationMode,
    ) -> Result<(), String> {
        let media_id =
            self.media_ids.get(&item.media_id).copied().ok_or_else(|| {
                format!("Media {} was not recorded in the catalog", item.source_name)
            })?;
        self.catalog
            .update_media_fingerprints(media_id, Some(digest.quick().hex), Some(digest.full().hex))
            .map_err(|e| e.to_string())?;
        self.catalog
            .record_import(
                self.session_id,
                ImportInput {
                    media_id,
                    destination_path: copy.final_destination.display().to_string(),
                    destination_size: Some(digest.size()),
                    copy_completed_at: Some(now()),
                    verification_method: Some(format!("{verification:?}")),
                    verified_at: Some(now()),
                    status: CatalogImportStatus::Verified,
                    error: None,
                },
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_import(
    source: Arc<dyn MediaSource>,
    plan: ImportPlan,
    cancel: CancellationToken,
    catalog: CatalogHandle,
    source_id: Option<i64>,
    media_ids: HashMap<MediaId, i64>,
    generation: ScanGeneration,
    sender: Sender<WorkMessage>,
) {
    let Some(source_id) = source_id else {
        let _ = sender.send(WorkMessage::ImportDone(
            generation,
            Err("source was not recorded".into()),
        ));
        return;
    };
    let session = match catalog.begin_session(source_id, None, now()) {
        Ok(s) => s,
        Err(e) => {
            let _ = sender.send(WorkMessage::ImportDone(generation, Err(e.to_string())));
            return;
        }
    };
    let mut recorder = Recorder {
        catalog: catalog.clone(),
        session_id: session.id,
        media_ids,
    };
    let result = ImportEngine::execute(source.as_ref(), &plan, &cancel, &mut recorder, |p| {
        let event = AppEvent::ImportProgress {
            generation,
            progress: captureport_core::ImportProgress {
                session_id: session.id as u64,
                completed: p.files_completed as u64,
                total: p.files_total as u64,
                bytes_copied: p.overall_bytes,
                bytes_total: p.overall_total,
                media_id: p.media_id,
                destination: p.destination,
                current_file_bytes: p.current_file_bytes,
                current_file_total: p.current_file_total,
                bytes_per_second: p.bytes_per_second,
            },
        };
        let _ = sender.send(WorkMessage::Event(Box::new(event)));
    });
    let (status, message) = match result {
        Ok(result) => {
            let _ = sender.send(WorkMessage::ImportResult(generation, result.clone()));
            for item in &result.items {
                if let Some(media_id) = recorder.media_ids.get(&item.media_id).copied() {
                    for copy in &item.copies {
                        if matches!(
                            copy.state,
                            captureport_ingest::ImportItemState::Failed
                                | captureport_ingest::ImportItemState::Cancelled
                        ) {
                            let _ = catalog.record_import(
                                session.id,
                                ImportInput {
                                    media_id,
                                    destination_path: copy.destination.display().to_string(),
                                    destination_size: None,
                                    copy_completed_at: None,
                                    verification_method: Some(format!("{:?}", plan.verification)),
                                    verified_at: None,
                                    status: if copy.state
                                        == captureport_ingest::ImportItemState::Cancelled
                                    {
                                        CatalogImportStatus::Cancelled
                                    } else {
                                        CatalogImportStatus::Failed
                                    },
                                    error: copy.error.as_ref().map(ToString::to_string),
                                },
                            );
                        }
                    }
                }
            }
            let completed = result
                .items
                .iter()
                .flat_map(|item| &item.copies)
                .filter(|copy| copy.state == captureport_ingest::ImportItemState::Completed)
                .count();
            let failed = result
                .items
                .iter()
                .flat_map(|item| &item.copies)
                .filter(|copy| copy.state == captureport_ingest::ImportItemState::Failed)
                .count();
            for item in &result.items {
                let completed_required = deletable_sources(&plan, &result, false)
                    .iter()
                    .any(|planned| planned.media_id == item.media_id);
                if completed_required {
                    let _ = sender.send(WorkMessage::Event(Box::new(
                        AppEvent::ImportStatusChanged {
                            generation,
                            media_id: item.media_id,
                            status: captureport_core::ImportStatus::Imported,
                            prior_import: None,
                        },
                    )));
                }
            }
            let status = if result.cancelled {
                captureport_catalog::SessionStatus::Cancelled
            } else if failed > 0 {
                captureport_catalog::SessionStatus::Partial
            } else {
                captureport_catalog::SessionStatus::Completed
            };
            (
                status,
                Ok(format!("Imported {completed} copy/copies; {failed} failed")),
            )
        }
        Err(error) => (
            captureport_catalog::SessionStatus::Failed,
            Err(error.to_string()),
        ),
    };
    let completion = catalog.complete_session(
        session.id,
        Some(now()),
        status,
        message.as_ref().err().cloned(),
    );
    let message = match completion {
        Ok(_) => message,
        Err(error) => Err(format!(
            "Copies finished, but session history could not be saved: {error}"
        )),
    };
    let _ = sender.send(WorkMessage::ImportDone(generation, message));
}
pub(crate) fn run_reconcile(
    roots: &[PathBuf],
    catalog: &CatalogHandle,
    cancel: &CancellationToken,
) -> Result<String, String> {
    let source = catalog
        .upsert_source(
            captureport_catalog::SourceIdentity {
                stable_id: Some("captureport:existing-library".into()),
                source_type: "ExistingLibrary".into(),
                manufacturer: None,
                model: None,
                serial: None,
                volume_uuid: None,
                alias: Some("Existing library".into()),
            },
            now(),
        )
        .map_err(|e| e.to_string())?;
    let session = catalog
        .begin_session(source.id, None, now())
        .map_err(|e| e.to_string())?;
    let report = captureport_ingest::reconcile_library(roots, cancel);
    let cancelled = report.cancelled;
    let mut recorded = 0usize;
    let mut errors = report.errors.len();
    for record in report.records {
        let media_type = match record.media_type {
            captureport_core::MediaType::Raw => captureport_catalog::MediaType::Raw,
            captureport_core::MediaType::Video => captureport_catalog::MediaType::Video,
            captureport_core::MediaType::Sidecar => captureport_catalog::MediaType::Sidecar,
            captureport_core::MediaType::Unknown => captureport_catalog::MediaType::Unknown,
            _ => captureport_catalog::MediaType::Photo,
        };
        let identity = captureport_catalog::MediaIdentity {
            source_id: source.id,
            source_path: record.path.display().to_string(),
            source_filename: record
                .path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
            source_size: record.size,
            capture_time: None,
        };
        let quick = record
            .quick
            .as_ref()
            .map(|fingerprint| fingerprint.hex.clone());
        if let Ok(Some(existing)) = catalog.lookup_media(identity.clone())
            && existing.quick_fingerprint == quick
            && catalog.lookup_imported(identity.clone()).unwrap_or(false)
        {
            continue;
        }
        let verified = quick.is_some();
        let result = catalog.record_reconciled_import(ReconciledImportInput {
            session_id: session.id,
            media: identity,
            media_type,
            observed_at: now(),
            quick_fingerprint: quick,
            content_hash: record
                .full
                .as_ref()
                .map(|fingerprint| fingerprint.hex.clone()),
            import: ImportInput {
                media_id: 0,
                destination_path: record.path.display().to_string(),
                destination_size: Some(record.size),
                copy_completed_at: Some(now()),
                verification_method: verified.then(|| "quick-blake3-v1".into()),
                verified_at: verified.then(now),
                status: if verified {
                    CatalogImportStatus::Verified
                } else {
                    CatalogImportStatus::Completed
                },
                error: None,
            },
        });
        if result.is_ok() {
            recorded += 1
        } else {
            errors += 1
        }
    }
    let was_cancelled = cancelled || cancel.is_cancelled();
    let status = if was_cancelled {
        captureport_catalog::SessionStatus::Cancelled
    } else if errors == 0 {
        captureport_catalog::SessionStatus::Completed
    } else {
        captureport_catalog::SessionStatus::Partial
    };
    catalog
        .complete_session(
            session.id,
            Some(now()),
            status,
            (errors > 0).then(|| format!("{errors} scan or catalog errors")),
        )
        .map_err(|e| e.to_string())?;
    Ok(format!(
        "Reconciled {recorded} files ({errors} errors{})",
        if was_cancelled { ", cancelled" } else { "" }
    ))
}
pub(crate) fn aliases_for_sources(
    sources: &[captureport_gphoto::discovery::DiscoveredSource],
    catalog: &CatalogHandle,
) -> HashMap<String, String> {
    sources
        .iter()
        .filter_map(|source| {
            let catalog_id = match source {
                captureport_gphoto::discovery::DiscoveredSource::Ptp(camera) => {
                    camera.stable_id.clone()
                }
                captureport_gphoto::discovery::DiscoveredSource::MountedFilesystem {
                    path,
                    stable_id,
                    volume_uuid,
                    ..
                } => volume_uuid
                    .as_ref()
                    .map(|uuid| format!("filesystem:uuid:{uuid}"))
                    .or_else(|| {
                        stable_id
                            .strip_prefix("mount:device:")
                            .map(|device| format!("filesystem:device:{device}"))
                    })
                    .unwrap_or_else(|| {
                        format!(
                            "filesystem:{}",
                            std::fs::canonicalize(path)
                                .unwrap_or_else(|_| path.clone())
                                .display()
                        )
                    }),
            };
            catalog
                .source_by_stable_id(catalog_id)
                .ok()
                .flatten()
                .and_then(|record| record.identity.alias)
                .map(|alias| (source.stable_id().to_string(), alias))
        })
        .collect()
}
/// Use the complete selection, including hidden items and selected bundle
/// members, while keeping genuine imported-history matches intact.
pub(crate) fn manual_mark_selection(
    state: &AppState,
    catalog_ids: &HashMap<MediaId, i64>,
) -> Result<Vec<(MediaId, i64)>, String> {
    state.items()
        .filter(|item| state.is_selected(item.id)
            && item.import_status != captureport_core::ImportStatus::Imported)
        .map(|item| {
            catalog_ids.get(&item.id).copied().map(|catalog_id| (item.id, catalog_id))
                .ok_or_else(|| format!(
                    "Cannot mark {} yet: its catalog record is unavailable. Rescan the source and try again.",
                    item.source_name
                ))
        })
        .collect()
}
pub(crate) fn now() -> String {
    Utc::now().to_rfc3339()
}
/// Required-copy completion drives import status; deletion confirmation must
/// additionally require every planned destination, matching the remover.
pub(crate) fn deletable_sources<'a>(
    plan: &'a ImportPlan,
    result: &captureport_ingest::ImportResult,
    all_destinations: bool,
) -> Vec<&'a PlannedImport> {
    plan.items
        .iter()
        .filter(|planned| {
            result
                .items
                .iter()
                .find(|item| item.media_id == planned.media_id)
                .is_some_and(|item| {
                    planned
                        .copies
                        .iter()
                        .filter(|copy| all_destinations || copy.required)
                        .all(|planned_copy| {
                            item.copies.iter().any(|actual| {
                                actual.destination == planned_copy.final_destination
                                    && actual.state
                                        == captureport_ingest::ImportItemState::Completed
                            })
                        })
                })
        })
        .collect()
}

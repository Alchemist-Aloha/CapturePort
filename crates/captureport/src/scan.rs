//! Background source scanning and catalog classification.
//!
//! This module deliberately owns all source and metadata I/O.  The GPUI layer
//! receives small messages and only applies them to its state; enumeration is
//! allowed to continue while the bounded metadata queue catches up.

use captureport_catalog::{
    CatalogCommand, CatalogHandle, MediaIdentity, SourceIdentity as CatalogSourceIdentity,
};
use captureport_core::{
    AppEvent, FakeSourceBuilder, MediaId, MediaItem, MediaSource, MediaType, ScanContext,
    ScanGeneration, SourceError,
};
use captureport_ingest::quick_source;
use captureport_media::FilesystemSource;
use captureport_metadata::MetadataService;
use chrono::Utc;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex, mpsc},
    thread,
    time::UNIX_EPOCH,
};

#[derive(Clone, Debug)]
pub(crate) enum SourceRequest {
    Filesystem(PathBuf),
    Demo(FakeSourceBuilder),
    Camera(captureport_gphoto::CameraDescriptor),
}

pub(crate) enum ScanMessage {
    Source(Arc<dyn MediaSource>, Option<i64>, Option<String>),
    CatalogMedia(MediaId, i64),
    /// Source modification time captured on a worker for thumbnail cache keys.
    ThumbnailInfo(MediaId, u64),
    Event(Box<AppEvent>),
    Finished(Result<(), SourceError>),
}

struct MetadataJob {
    source: Arc<dyn MediaSource>,
    item: MediaItem,
    filesystem_path: Option<PathBuf>,
    catalog: CatalogHandle,
    catalog_source_id: Option<i64>,
    generation: ScanGeneration,
    sender: mpsc::Sender<ScanMessage>,
}

/// Scan one source on a worker thread.  The first event for every item is
/// emitted during enumeration; metadata, fingerprints, and catalog writes run
/// through a bounded two-worker queue.
pub(crate) fn scan_source(
    request: SourceRequest,
    scan: ScanContext,
    sender: mpsc::Sender<ScanMessage>,
    catalog: CatalogHandle,
) {
    let filesystem_root = match &request {
        SourceRequest::Filesystem(path) => Some(path.clone()),
        SourceRequest::Demo(_) | SourceRequest::Camera(_) => None,
    };
    let source = match open_source(request) {
        Ok(source) => source,
        Err(error) => {
            let _ = sender.send(ScanMessage::Finished(Err(error)));
            return;
        }
    };
    let identity = source.identity();
    let catalog_source = catalog
        .upsert_source(
            CatalogSourceIdentity {
                stable_id: identity.stable_id.clone(),
                source_type: format!("{:?}", identity.source_type),
                manufacturer: identity.manufacturer.clone(),
                model: identity.model.clone(),
                serial: identity.serial.clone(),
                volume_uuid: identity.volume_uuid.clone(),
                alias: None,
            },
            now(),
        )
        .ok();
    let catalog_source_id = catalog_source.as_ref().map(|record| record.id);
    let catalog_alias = catalog_source
        .as_ref()
        .and_then(|record| record.identity.alias.clone());
    let _ = sender.send(ScanMessage::Source(
        source.clone(),
        catalog_source_id,
        catalog_alias,
    ));
    let _ = sender.send(ScanMessage::Event(Box::new(AppEvent::SourceDetected {
        generation: scan.generation,
        source: identity,
    })));

    let (jobs_tx, jobs_rx) = mpsc::sync_channel::<MetadataJob>(64);
    let jobs_rx = Arc::new(Mutex::new(jobs_rx));
    let mut workers = Vec::with_capacity(2);
    for _ in 0..2 {
        let jobs_rx = Arc::clone(&jobs_rx);
        workers.push(thread::spawn(move || {
            loop {
                let job = match jobs_rx.lock().expect("metadata queue poisoned").recv() {
                    Ok(job) => job,
                    Err(_) => break,
                };
                process_job(job);
            }
        }));
    }

    let enumerate_result = source.enumerate(&scan, &mut |item| {
        if scan.is_cancelled() {
            return Err(SourceError::Cancelled);
        }
        sender
            .send(ScanMessage::Event(Box::new(AppEvent::MediaDiscovered {
                generation: scan.generation,
                item: item.clone(),
            })))
            .map_err(|_| SourceError::Callback("scan receiver closed".into()))?;
        let filesystem_path = filesystem_root
            .as_ref()
            .map(|root| root.join(&item.source_path));
        jobs_tx
            .send(MetadataJob {
                source: source.clone(),
                item,
                filesystem_path,
                catalog: catalog.clone(),
                catalog_source_id,
                generation: scan.generation,
                sender: sender.clone(),
            })
            .map_err(|_| SourceError::Callback("metadata queue closed".into()))?;
        Ok(())
    });
    drop(jobs_tx);
    for worker in workers {
        let _ = worker.join();
    }
    let _ = sender.send(ScanMessage::Finished(enumerate_result));
}

fn open_source(request: SourceRequest) -> Result<Arc<dyn MediaSource>, SourceError> {
    match request {
        SourceRequest::Filesystem(path) => {
            FilesystemSource::new(path).map(|source| Arc::new(source) as Arc<dyn MediaSource>)
        }
        SourceRequest::Demo(builder) => Ok(Arc::new(builder.build())),
        SourceRequest::Camera(camera) => captureport_gphoto::GPhotoSource::connect(camera)
            .map(|source| Arc::new(source) as Arc<dyn MediaSource>)
            .map_err(|error| {
                tracing::warn!(operation=%error.operation,code=error.code,"camera connection failed");
                SourceError::Io("Could not connect to the camera. Check its connection and close other camera applications.".into())
            }),
    }
}

fn process_job(job: MetadataJob) {
    let MetadataJob {
        source,
        item,
        filesystem_path,
        catalog,
        catalog_source_id,
        generation,
        sender,
    } = job;
    let modified = filesystem_path
        .as_ref()
        .and_then(|path| std::fs::metadata(path).ok())
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs());
    if let Some(modified) = modified {
        let _ = sender.send(ScanMessage::ThumbnailInfo(item.id, modified));
    }

    let (capture_time, metadata_result) = match (&filesystem_path, &item.metadata) {
        (_, captureport_core::MetadataState::Failed(error)) => (None, Err(error.clone())),
        (Some(path), _) => match MetadataService::extract(path, item.media_type) {
            Ok(metadata) => (metadata.capture_time.clone(), Ok(Some(metadata))),
            Err(error) => (None, Err(error.to_string())),
        },
        (None, _) => (None, Ok(None)),
    };
    match metadata_result {
        Ok(Some(metadata)) => {
            let _ = sender.send(ScanMessage::Event(Box::new(AppEvent::MetadataReady {
                generation,
                media_id: item.id,
                metadata: metadata.core_metadata(),
            })));
        }
        Ok(None) => {}
        Err(error) => {
            let _ = sender.send(ScanMessage::Event(Box::new(AppEvent::MetadataFailed {
                generation,
                media_id: item.id,
                error,
            })));
        }
    }

    // PTP range reads may spool an entire object; defer strong matching until
    // import rather than downloading every camera item during browsing.
    let quick = if filesystem_path.is_some() {
        quick_source(&*source, &item.locator, item.size).ok()
    } else {
        None
    };
    let catalog_media_id = catalog_source_id.and_then(|source_id| {
        let media = MediaIdentity {
            source_id,
            source_path: item.source_path.clone(),
            source_filename: item.source_name.clone(),
            source_size: item.size,
            capture_time,
        };
        let media_type = catalog_media_type(item.media_type);
        // Check before upserting this observation. A freshly inserted row
        // must not make every new item appear to be a duplicate.
        let existing = catalog.lookup_media(media.clone()).ok().flatten();
        let imported_by_identity = catalog.lookup_imported(media.clone()).unwrap_or(false);
        let record = catalog
            .execute(CatalogCommand::UpsertMedia {
                media: media.clone(),
                media_type,
                observed_at: now(),
                quick_fingerprint: quick.as_ref().map(|fp| fp.hex.clone()),
                content_hash: None,
            })
            .ok();
        let mut status = captureport_core::ImportStatus::New;
        if let Some(fingerprint) = &quick {
            if catalog
                .lookup_imported_fingerprint(item.size, fingerprint.hex.clone(), None)
                .unwrap_or(false)
            {
                status = captureport_core::ImportStatus::Imported;
            } else if existing.is_some() {
                status = captureport_core::ImportStatus::PossibleDuplicate;
            }
        } else if imported_by_identity || existing.is_some() {
            status = captureport_core::ImportStatus::PossibleDuplicate;
        }
        let _ = sender.send(ScanMessage::Event(Box::new(
            AppEvent::ImportStatusChanged {
                generation,
                media_id: item.id,
                status,
            },
        )));
        record.and_then(|response| match response {
            captureport_catalog::CatalogResponse::Media(record) => Some(record.id),
            _ => None,
        })
    });
    if let Some(catalog_media_id) = catalog_media_id {
        let _ = sender.send(ScanMessage::CatalogMedia(item.id, catalog_media_id));
    }
}

fn catalog_media_type(media_type: MediaType) -> captureport_catalog::MediaType {
    match media_type {
        MediaType::Video => captureport_catalog::MediaType::Video,
        MediaType::Raw => captureport_catalog::MediaType::Raw,
        MediaType::Sidecar => captureport_catalog::MediaType::Sidecar,
        MediaType::Unknown => captureport_catalog::MediaType::Unknown,
        MediaType::Jpeg | MediaType::Heif | MediaType::Png | MediaType::Tiff => {
            captureport_catalog::MediaType::Photo
        }
    }
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

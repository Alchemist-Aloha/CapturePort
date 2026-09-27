mod logging;
mod paths;
mod scan;
mod text_input;

use captureport_catalog::{
    CatalogHandle, ImportInput, ImportStatus as CatalogImportStatus, IncompleteSession,
    ReconciledImportInput, SessionDetail,
};
use captureport_core::{
    AppEvent, AppState, CancellationToken, FakeSourceBuilder, FakeSourceScenario, MediaFilter,
    MediaId, MediaLocator, MediaSort, MediaSource, ScanContext, ScanGeneration, SourceError,
    SourceId, SourceType,
};
use captureport_ingest::{
    BackupRule, BundlePolicy, CollisionPolicy, CopyDigest, DestinationRule, Grouping, ImportEngine,
    ImportPlan, ImportPlanner, ImportPreset, ImportRecorder, PlanInput, PlannedCopy, PlannedImport,
    Template, VerificationMode,
};
use captureport_metadata::{ThumbnailPipeline, ThumbnailRequest, ThumbnailState};
use chrono::{DateTime, FixedOffset, NaiveDateTime, TimeZone, Utc};
use gpui::{
    App, Application, Bounds, Context, Entity, FocusHandle, Focusable, KeyBinding,
    PathPromptOptions, Timer, Window, WindowBounds, WindowOptions, actions, div, img, prelude::*,
    px, rgb, size, uniform_list,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender, SyncSender},
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

actions!(
    captureport,
    [
        OpenFolder,
        OpenDemo,
        SelectAll,
        SelectAllNew,
        SelectNone,
        ImportSelected,
        CancelImport,
        ShowHistory,
        ReconcileLibrary,
        CancelReconcile,
        AdjustClock,
        DiscoverSources
    ]
);
use scan::{ScanMessage, SourceRequest, scan_source};
enum WorkMessage {
    Event(Box<AppEvent>),
    Plan(u64, ImportPlan),
    History(Vec<HistoryEntry>),
    Sources(
        Vec<captureport_gphoto::discovery::DiscoveredSource>,
        HashMap<String, String>,
    ),
    Recovery(Vec<IncompleteSession>, Vec<captureport_ingest::PartialFile>),
    PartialCleaned(PathBuf),
    ImportResult(captureport_ingest::ImportResult),
    CacheCleared(usize),
    SourceAlias(Option<String>),
    ImportDone(Result<String, String>),
    ReconcileDone(Result<String, String>),
    Done(Result<String, String>),
}
struct HistoryEntry {
    detail: SessionDetail,
    source_name: String,
}
struct CameraPreviewJob {
    generation: u64,
    id: MediaId,
    source: Arc<dyn MediaSource>,
    locator: MediaLocator,
    cache_path: PathBuf,
    cancellation: CancellationToken,
}
struct CameraPreviewResult {
    generation: u64,
    id: MediaId,
    cache_path: Option<PathBuf>,
}
#[derive(Clone, Copy, Eq, PartialEq)]
enum Page {
    Browser,
    Preview,
    History,
    Recovery,
    Settings,
}
struct SettingsInputs {
    photo_root: Entity<text_input::TextInput>,
    photo_folder: Entity<text_input::TextInput>,
    video_root: Entity<text_input::TextInput>,
    video_folder: Entity<text_input::TextInput>,
    filename: Entity<text_input::TextInput>,
    clock_seconds: Entity<text_input::TextInput>,
    timezone_seconds: Entity<text_input::TextInput>,
    gap_minutes: Entity<text_input::TextInput>,
    backup_photo_root: Entity<text_input::TextInput>,
    backup_video_root: Entity<text_input::TextInput>,
    source_alias: Entity<text_input::TextInput>,
}
struct Startup {
    catalog: CatalogHandle,
    cache_dir: PathBuf,
    config_path: PathBuf,
    ui_path: PathBuf,
    ui: UiPreferences,
    preset: ImportPreset,
    initial_source: Option<SourceRequest>,
    demo_importing: bool,
}
#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(default)]
struct UiPreferences {
    dark_mode: bool,
    thumbnail_size: u8,
}
impl Default for UiPreferences {
    fn default() -> Self {
        Self {
            dark_mode: false,
            thumbnail_size: 2,
        }
    }
}
impl SettingsInputs {
    fn new(preset: &ImportPreset, cx: &mut Context<Browser>) -> Self {
        let field = |value: String, cx: &mut Context<Browser>| {
            cx.new(|cx| text_input::TextInput::new(value, "", cx))
        };
        Self {
            photo_root: field(preset.photo.root.display().to_string(), cx),
            photo_folder: field(preset.photo.folder_template.clone(), cx),
            video_root: field(preset.video.root.display().to_string(), cx),
            video_folder: field(preset.video.folder_template.clone(), cx),
            filename: field(preset.filename_template.clone(), cx),
            clock_seconds: field(preset.time_correction.offset_seconds.to_string(), cx),
            timezone_seconds: field(
                preset
                    .time_correction
                    .assumed_utc_offset_seconds
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
                cx,
            ),
            gap_minutes: field(
                match preset.grouping {
                    Grouping::TimeGap { threshold_minutes } => threshold_minutes.to_string(),
                    _ => "30".into(),
                },
                cx,
            ),
            backup_photo_root: field(
                preset
                    .backup
                    .as_ref()
                    .map(|b| b.photo.root.display().to_string())
                    .unwrap_or_default(),
                cx,
            ),
            backup_video_root: field(
                preset
                    .backup
                    .as_ref()
                    .map(|b| b.video.root.display().to_string())
                    .unwrap_or_default(),
                cx,
            ),
            source_alias: field(String::new(), cx),
        }
    }
    fn sync(&self, preset: &ImportPreset, cx: &mut Context<Browser>) {
        self.photo_root.update(cx, |input, cx| {
            input.set_value(preset.photo.root.display().to_string(), cx)
        });
        self.photo_folder.update(cx, |input, cx| {
            input.set_value(preset.photo.folder_template.clone(), cx)
        });
        self.video_root.update(cx, |input, cx| {
            input.set_value(preset.video.root.display().to_string(), cx)
        });
        self.video_folder.update(cx, |input, cx| {
            input.set_value(preset.video.folder_template.clone(), cx)
        });
        self.filename.update(cx, |input, cx| {
            input.set_value(preset.filename_template.clone(), cx)
        });
        self.clock_seconds.update(cx, |input, cx| {
            input.set_value(preset.time_correction.offset_seconds.to_string(), cx)
        });
        self.timezone_seconds.update(cx, |input, cx| {
            input.set_value(
                preset
                    .time_correction
                    .assumed_utc_offset_seconds
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
                cx,
            )
        });
        self.gap_minutes.update(cx, |input, cx| {
            input.set_value(
                match preset.grouping {
                    Grouping::TimeGap { threshold_minutes } => threshold_minutes.to_string(),
                    _ => "30".into(),
                },
                cx,
            )
        });
        self.backup_photo_root.update(cx, |input, cx| {
            input.set_value(
                preset
                    .backup
                    .as_ref()
                    .map(|b| b.photo.root.display().to_string())
                    .unwrap_or_default(),
                cx,
            )
        });
        self.backup_video_root.update(cx, |input, cx| {
            input.set_value(
                preset
                    .backup
                    .as_ref()
                    .map(|b| b.video.root.display().to_string())
                    .unwrap_or_default(),
                cx,
            )
        });
    }
    fn read(
        &self,
        current: &ImportPreset,
        backup_required: bool,
        cx: &App,
    ) -> Result<ImportPreset, String> {
        let root = |value: String| -> Result<PathBuf, String> {
            let path = PathBuf::from(value);
            if path.is_absolute() {
                Ok(path)
            } else {
                Err("Destination roots must be absolute paths".into())
            }
        };
        let photo_folder = self.photo_folder.read(cx).value();
        let video_folder = self.video_folder.read(cx).value();
        let filename = self.filename.read(cx).value();
        for template in [&photo_folder, &video_folder, &filename] {
            Template::parse(template).map_err(|e| e.to_string())?;
        }
        let clock_seconds = self
            .clock_seconds
            .read(cx)
            .value()
            .parse::<i64>()
            .map_err(|_| "Clock correction must be a number of seconds".to_string())?;
        let timezone = self.timezone_seconds.read(cx).value();
        let assumed_utc_offset_seconds = if timezone.trim().is_empty() {
            None
        } else {
            let value = timezone
                .parse::<i32>()
                .map_err(|_| "Timezone offset must be seconds east of UTC".to_string())?;
            if !(-86_400..=86_400).contains(&value) {
                return Err("Timezone offset is out of range".into());
            }
            Some(value)
        };
        let backup_photo = self.backup_photo_root.read(cx).value();
        let backup_video = self.backup_video_root.read(cx).value();
        let backup = if backup_photo.trim().is_empty() && backup_video.trim().is_empty() {
            None
        } else {
            if backup_photo.trim().is_empty() || backup_video.trim().is_empty() {
                return Err("Set both backup roots or clear both".into());
            }
            Some(BackupRule {
                photo: DestinationRule {
                    root: root(backup_photo)?,
                    folder_template: photo_folder.clone(),
                },
                video: DestinationRule {
                    root: root(backup_video)?,
                    folder_template: video_folder.clone(),
                },
                required: backup_required,
            })
        };
        let mut preset = current.clone();
        preset.name = "Custom".into();
        preset.photo = DestinationRule {
            root: root(self.photo_root.read(cx).value())?,
            folder_template: photo_folder,
        };
        preset.video = DestinationRule {
            root: root(self.video_root.read(cx).value())?,
            folder_template: video_folder,
        };
        preset.filename_template = filename;
        preset.time_correction.offset_seconds = clock_seconds;
        preset.time_correction.assumed_utc_offset_seconds = assumed_utc_offset_seconds;
        if matches!(preset.grouping, Grouping::TimeGap { .. }) {
            let threshold_minutes = self
                .gap_minutes
                .read(cx)
                .value()
                .parse::<u32>()
                .map_err(|_| "Session gap must be a number of minutes".to_string())?;
            if threshold_minutes == 0 {
                return Err("Session gap must be greater than zero".into());
            }
            preset.grouping = Grouping::TimeGap { threshold_minutes };
        }
        preset.backup = backup;
        Ok(preset)
    }
}

struct Browser {
    state: AppState,
    visible_ids: Vec<MediaId>,
    bundles: HashMap<MediaId, Vec<MediaId>>,
    bundle_members: HashSet<MediaId>,
    receiver: Option<Receiver<ScanMessage>>,
    work_receivers: Vec<Receiver<WorkMessage>>,
    cancellation: Option<CancellationToken>,
    import_cancellation: Option<CancellationToken>,
    reconcile_cancellation: Option<CancellationToken>,
    last_import_result: Option<captureport_ingest::ImportResult>,
    deletion_armed: bool,
    source: Option<Arc<dyn MediaSource>>,
    filesystem_root: Option<PathBuf>,
    catalog: CatalogHandle,
    catalog_source_id: Option<i64>,
    source_alias: Option<String>,
    catalog_media_ids: HashMap<MediaId, i64>,
    preset: ImportPreset,
    backup_required_choice: bool,
    settings: SettingsInputs,
    config_path: PathBuf,
    ui_path: PathBuf,
    ui: UiPreferences,
    plan: Option<ImportPlan>,
    plan_revision: u64,
    page: Page,
    planning: bool,
    history: Vec<HistoryEntry>,
    session_detail: Option<SessionDetail>,
    discovered_sources: Vec<captureport_gphoto::discovery::DiscoveredSource>,
    discovery_aliases: HashMap<String, String>,
    discovery_receiver: Receiver<(
        Vec<captureport_gphoto::discovery::DiscoveredSource>,
        HashMap<String, String>,
    )>,
    discovery_stop: Arc<AtomicBool>,
    incomplete_sessions: Vec<IncompleteSession>,
    partial_files: Vec<captureport_ingest::PartialFile>,
    thumbnails: ThumbnailPipeline,
    thumbnail_keys: HashMap<String, MediaId>,
    thumbnail_paths: HashMap<MediaId, PathBuf>,
    thumbnail_modified: HashMap<MediaId, u64>,
    requested_thumbnails: HashSet<MediaId>,
    camera_preview_sender: SyncSender<CameraPreviewJob>,
    camera_preview_receiver: Receiver<CameraPreviewResult>,
    camera_preview_scan_token: u128,
    cache_clearing: bool,
    cache_dir: PathBuf,
    generation: u64,
    scanning: bool,
    importing: bool,
    progress: Option<captureport_core::ImportProgress>,
    message: Option<String>,
    focus: FocusHandle,
}
impl Browser {
    fn new(window: &mut Window, cx: &mut Context<Self>, startup: Startup) -> Self {
        let Startup {
            catalog,
            cache_dir,
            config_path,
            ui_path,
            ui,
            preset,
            initial_source,
            demo_importing,
        } = startup;
        let focus = cx.focus_handle();
        window.focus(&focus);
        let settings = SettingsInputs::new(&preset, cx);
        let (camera_preview_sender, camera_preview_jobs) =
            mpsc::sync_channel::<CameraPreviewJob>(32);
        let (camera_preview_results, camera_preview_receiver) = mpsc::channel();
        thread::spawn(move || {
            while let Ok(job) = camera_preview_jobs.recv() {
                let cache_path = if job.cancellation.is_cancelled() {
                    None
                } else if job.cache_path.is_file() {
                    Some(job.cache_path)
                } else if let Ok(Some(bytes)) = job.source.preview(&job.locator) {
                    let temporary = job.cache_path.with_extension("jpg.partial");
                    let saved = image::load_from_memory(&bytes)
                        .and_then(|image| {
                            image
                                .thumbnail(320, 320)
                                .save_with_format(&temporary, image::ImageFormat::Jpeg)
                        })
                        .and_then(|_| {
                            std::fs::rename(&temporary, &job.cache_path)
                                .map_err(image::ImageError::IoError)
                        })
                        .is_ok();
                    if !saved {
                        let _ = std::fs::remove_file(&temporary);
                    }
                    saved.then_some(job.cache_path)
                } else {
                    None
                };
                if camera_preview_results
                    .send(CameraPreviewResult {
                        generation: job.generation,
                        id: job.id,
                        cache_path,
                    })
                    .is_err()
                {
                    break;
                }
            }
        });
        let backup_required_choice = preset.backup.as_ref().is_some_and(|b| b.required);
        let recovery_catalog = catalog.clone();
        let recovery_roots = vec![preset.photo.root.clone(), preset.video.root.clone()];
        cx.spawn(async move |this, cx| {
            loop {
                Timer::after(Duration::from_millis(16)).await;
                if this.update(cx, |this, cx| this.drain(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
        let (recovery_tx, recovery_rx) = mpsc::channel();
        thread::spawn(move || {
            let sessions = recovery_catalog.incomplete_sessions().unwrap_or_default();
            let partials = captureport_ingest::inspect_partials(&recovery_roots).partials;
            let _ = recovery_tx.send(WorkMessage::Recovery(sessions, partials));
        });
        let (discovery_tx, discovery_rx) = mpsc::channel();
        let discovery_catalog = catalog.clone();
        let discovery_stop = Arc::new(AtomicBool::new(false));
        let discovery_worker_stop = discovery_stop.clone();
        thread::spawn(move || {
            while !discovery_worker_stop.load(Ordering::Relaxed) {
                if let Ok(sources) = captureport_gphoto::discovery::discover()
                    && discovery_tx
                        .send((
                            sources.clone(),
                            aliases_for_sources(&sources, &discovery_catalog),
                        ))
                        .is_err()
                {
                    break;
                }
                for _ in 0..30 {
                    if discovery_worker_stop.load(Ordering::Relaxed) {
                        return;
                    }
                    thread::sleep(Duration::from_millis(100));
                }
            }
        });
        let mut browser = Self {
            state: AppState::new(),
            visible_ids: Vec::new(),
            bundles: HashMap::new(),
            bundle_members: HashSet::new(),
            receiver: None,
            work_receivers: vec![recovery_rx],
            cancellation: None,
            import_cancellation: None,
            reconcile_cancellation: None,
            last_import_result: None,
            deletion_armed: false,
            source: None,
            filesystem_root: None,
            catalog,
            catalog_source_id: None,
            source_alias: None,
            catalog_media_ids: HashMap::new(),
            preset,
            backup_required_choice,
            settings,
            config_path,
            ui_path,
            ui,
            plan: None,
            plan_revision: 0,
            page: Page::Browser,
            planning: false,
            history: Vec::new(),
            session_detail: None,
            discovered_sources: Vec::new(),
            discovery_aliases: HashMap::new(),
            discovery_receiver: discovery_rx,
            discovery_stop,
            incomplete_sessions: Vec::new(),
            partial_files: Vec::new(),
            thumbnails: ThumbnailPipeline::new(cache_dir.join("thumbnails"), 2, 128)
                .expect("thumbnail cache"),
            thumbnail_keys: HashMap::new(),
            thumbnail_paths: HashMap::new(),
            thumbnail_modified: HashMap::new(),
            requested_thumbnails: HashSet::new(),
            camera_preview_sender,
            camera_preview_receiver,
            camera_preview_scan_token: 0,
            cache_clearing: false,
            cache_dir,
            generation: 0,
            scanning: false,
            importing: false,
            progress: None,
            message: None,
            focus,
        };
        if let Some(request) = initial_source {
            browser.scan(request, cx);
        }
        if demo_importing {
            browser.importing = true;
            browser.progress = Some(captureport_core::ImportProgress {
                session_id: 0,
                completed: 25,
                total: 100,
                bytes_copied: 512_000_000,
            });
        }
        browser
    }
    fn save_ui(&mut self) {
        let result = (|| -> Result<(), String> {
            let data = serde_json::to_vec_pretty(&self.ui).map_err(|e| e.to_string())?;
            let temporary = self.ui_path.with_extension("json.tmp");
            std::fs::write(&temporary, data).map_err(|e| e.to_string())?;
            std::fs::rename(temporary, &self.ui_path).map_err(|e| e.to_string())?;
            Ok(())
        })();
        if let Err(error) = result {
            self.message = Some(format!("Appearance: {error}"));
        }
    }
    fn set_thumbnail_size(&mut self, size: u8, cx: &mut Context<Self>) {
        let size = size.min(4);
        if self.ui.thumbnail_size != size {
            self.ui.thumbnail_size = size;
            self.save_ui();
            cx.notify();
        }
    }
    fn toggle_dark_mode(&mut self, cx: &mut Context<Self>) {
        self.ui.dark_mode = !self.ui.dark_mode;
        self.save_ui();
        cx.notify();
    }
    fn drain(&mut self, cx: &mut Context<Self>) {
        let mut changed = false;
        while let Ok((sources, aliases)) = self.discovery_receiver.try_recv() {
            self.discovered_sources = sources;
            self.discovery_aliases = aliases;
            changed = true;
        }
        if let Some(r) = &self.receiver {
            for _ in 0..512 {
                match r.try_recv() {
                    Ok(ScanMessage::Source(s, id, alias)) => {
                        self.source_alias = alias.clone();
                        self.settings.source_alias.update(cx, |input, cx| {
                            input.set_value(alias.unwrap_or_default(), cx)
                        });
                        self.source = Some(s);
                        self.catalog_source_id = id
                    }
                    Ok(ScanMessage::CatalogMedia(core_id, catalog_id)) => {
                        self.catalog_media_ids.insert(core_id, catalog_id);
                    }
                    Ok(ScanMessage::ThumbnailInfo(id, modified)) => {
                        self.thumbnail_modified.insert(id, modified);
                    }
                    Ok(ScanMessage::Event(e)) => changed |= self.state.apply_event(*e),
                    Ok(ScanMessage::Finished(result)) => {
                        self.scanning = false;
                        self.rebuild_bundles();
                        if let Err(e) = result
                            && e != SourceError::Cancelled
                        {
                            self.message = Some(format!("Scan stopped: {e}"));
                        }
                        changed = true;
                        break;
                    }
                    Err(_) => break,
                }
            }
        }
        let mut index = 0;
        while index < self.work_receivers.len() {
            let mut disconnected = false;
            for _ in 0..256 {
                match self.work_receivers[index].try_recv() {
                    Ok(WorkMessage::Event(e)) => {
                        if let AppEvent::ImportProgress { progress, .. } = &*e {
                            self.progress = Some(progress.clone());
                        }
                        changed |= self.state.apply_event(*e)
                    }
                    Ok(WorkMessage::Plan(revision, plan)) if revision == self.plan_revision => {
                        self.planning = false;
                        self.message = Some(format!(
                            "Review {} planned item(s) before importing",
                            plan.items.len()
                        ));
                        self.plan = Some(plan);
                        self.page = Page::Preview;
                        changed = true;
                    }
                    Ok(WorkMessage::Plan(_, _)) => {}
                    Ok(WorkMessage::History(sessions)) => {
                        self.history = sessions;
                        self.session_detail = None;
                        self.page = Page::History;
                        changed = true;
                    }
                    Ok(WorkMessage::Sources(sources, aliases)) => {
                        self.discovered_sources = sources;
                        self.discovery_aliases = aliases;
                        changed = true;
                    }
                    Ok(WorkMessage::Recovery(sessions, partials)) => {
                        self.incomplete_sessions = sessions;
                        self.partial_files = partials;
                        if !self.incomplete_sessions.is_empty() || !self.partial_files.is_empty() {
                            self.page = Page::Recovery;
                            self.message = Some("An unfinished import needs review".into());
                        }
                        changed = true;
                    }
                    Ok(WorkMessage::PartialCleaned(path)) => {
                        self.partial_files.retain(|partial| partial.path != path);
                        self.message = Some(format!(
                            "Removed incomplete CapturePort file: {}",
                            path.display()
                        ));
                        changed = true;
                    }
                    Ok(WorkMessage::ImportResult(result)) => {
                        self.last_import_result = Some(result);
                        changed = true;
                    }
                    Ok(WorkMessage::CacheCleared(count)) => {
                        self.cache_clearing = false;
                        self.thumbnail_paths.clear();
                        self.thumbnail_keys.clear();
                        self.requested_thumbnails.clear();
                        self.message = Some(format!("Cleared {count} cached thumbnail(s)"));
                        changed = true;
                    }
                    Ok(WorkMessage::SourceAlias(alias)) => {
                        self.source_alias = alias;
                        self.message = Some("Source alias saved".into());
                        changed = true;
                    }
                    Ok(WorkMessage::Done(v)) => {
                        self.cache_clearing = false;
                        self.message = Some(v.unwrap_or_else(|e| format!("Operation failed: {e}")));
                        changed = true;
                    }
                    Ok(WorkMessage::ImportDone(v)) => {
                        self.importing = false;
                        self.import_cancellation = None;
                        self.message = Some(v.unwrap_or_else(|e| format!("Import failed: {e}")));
                        changed = true;
                    }
                    Ok(WorkMessage::ReconcileDone(v)) => {
                        self.reconcile_cancellation = None;
                        self.message =
                            Some(v.unwrap_or_else(|e| format!("Reconciliation failed: {e}")));
                        changed = true;
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        disconnected = true;
                        break;
                    }
                }
            }
            if disconnected {
                self.work_receivers.swap_remove(index);
            } else {
                index += 1;
            }
        }
        while let Some(result) = self.thumbnails.try_recv() {
            if result.state == ThumbnailState::Ready
                && let Some(id) = self.thumbnail_keys.get(&result.key).copied()
            {
                self.thumbnail_paths.insert(
                    id,
                    self.cache_dir
                        .join("thumbnails")
                        .join(format!("{}.jpg", result.key)),
                );
                changed = true;
            }
        }
        while let Ok(result) = self.camera_preview_receiver.try_recv() {
            if result.generation == self.generation {
                if let Some(path) = result.cache_path {
                    self.thumbnail_paths.insert(result.id, path);
                }
                changed = true;
            }
        }
        if changed {
            self.refresh();
            cx.notify();
        }
    }
    fn refresh(&mut self) {
        self.visible_ids = self
            .state
            .visible_items()
            .into_iter()
            .map(|i| i.id)
            .filter(|id| !self.bundle_members.contains(id))
            .collect();
    }
    fn invalidate_plan(&mut self) {
        self.plan_revision = self.plan_revision.wrapping_add(1);
        self.planning = false;
        self.plan = None;
        self.last_import_result = None;
        self.deletion_armed = false;
    }
    fn rebuild_bundles(&mut self) {
        self.bundles.clear();
        self.bundle_members.clear();
        let items = self.state.items().cloned().collect::<Vec<_>>();
        let grouped = captureport_ingest::group_media(&items);
        for bundle in grouped
            .bundles
            .into_iter()
            .filter(|bundle| bundle.members.len() > 1)
        {
            self.bundle_members.extend(
                bundle
                    .members
                    .iter()
                    .copied()
                    .filter(|id| *id != bundle.primary),
            );
            self.bundles.insert(bundle.primary, bundle.members);
        }
    }
    fn toggle_bundle(&mut self, id: MediaId, cx: &mut Context<Self>) {
        let selected = !self.state.is_selected(id);
        if let Some(members) = self.bundles.get(&id) {
            for member in members {
                self.state.select(*member, selected);
            }
        } else {
            self.state.select(id, selected);
        }
        self.invalidate_plan();
        self.last_import_result = None;
        self.deletion_armed = false;
        self.page = Page::Browser;
        cx.notify();
    }
    fn scan(&mut self, request: SourceRequest, cx: &mut Context<Self>) {
        if self.importing {
            self.message =
                Some("Finish or cancel the current import before changing sources".into());
            cx.notify();
            return;
        }
        if let Some(t) = &self.cancellation {
            t.cancel();
        }
        self.generation += 1;
        self.camera_preview_scan_token = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos());
        let g = ScanGeneration(self.generation);
        self.state.apply_event(AppEvent::ScanStarted {
            generation: g,
            source_id: SourceId(0),
        });
        self.visible_ids.clear();
        self.bundles.clear();
        self.bundle_members.clear();
        self.source = None;
        self.source_alias = None;
        self.filesystem_root = match &request {
            SourceRequest::Filesystem(path) => Some(path.clone()),
            _ => None,
        };
        self.catalog_media_ids.clear();
        self.invalidate_plan();
        self.page = Page::Browser;
        self.planning = false;
        self.thumbnail_keys.clear();
        self.thumbnail_paths.clear();
        self.thumbnail_modified.clear();
        self.requested_thumbnails.clear();
        self.thumbnails.cancel();
        self.thumbnails.reset_cancellation();
        self.scanning = true;
        self.message = None;
        let token = CancellationToken::new();
        let scan = ScanContext::new(g, token.clone());
        self.cancellation = Some(token);
        let (tx, rx) = mpsc::channel();
        self.receiver = Some(rx);
        let cat = self.catalog.clone();
        thread::spawn(move || scan_source(request, scan, tx, cat));
        cx.notify();
    }
    fn open_folder(&mut self, _: &OpenFolder, _: &mut Window, cx: &mut Context<Self>) {
        let picker = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Choose a media folder".into()),
        });
        cx.spawn(async move |this, cx| match picker.await {
            Ok(Ok(Some(paths))) => {
                if let Some(path) = paths.into_iter().next() {
                    let _ = this.update(cx, |this, cx| {
                        this.scan(SourceRequest::Filesystem(path), cx)
                    });
                }
            }
            Ok(Ok(None)) => {}
            Ok(Err(error)) => {
                let _ = this.update(cx, |this, cx| {
                    this.message = Some(format!("Could not choose a folder: {error}"));
                    cx.notify();
                });
            }
            Err(error) => {
                let _ = this.update(cx, |this, cx| {
                    this.message = Some(format!("Could not open the folder picker: {error}"));
                    cx.notify();
                });
            }
        })
        .detach();
    }
    fn open_demo(&mut self, _: &OpenDemo, _: &mut Window, cx: &mut Context<Self>) {
        self.scan(
            SourceRequest::Demo(FakeSourceBuilder::new().scenario(FakeSourceScenario::LargeCard)),
            cx,
        )
    }
    fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.state.select_all(true);
        self.invalidate_plan();
        self.page = Page::Browser;
        cx.notify()
    }
    fn select_new(&mut self, _: &SelectAllNew, _: &mut Window, cx: &mut Context<Self>) {
        self.state.select_all_new();
        self.invalidate_plan();
        self.page = Page::Browser;
        cx.notify()
    }
    fn select_none(&mut self, _: &SelectNone, _: &mut Window, cx: &mut Context<Self>) {
        self.state.select_all(false);
        self.invalidate_plan();
        self.page = Page::Browser;
        cx.notify()
    }
    fn filter(&mut self, f: MediaFilter, cx: &mut Context<Self>) {
        self.state.filter = f;
        self.invalidate_plan();
        self.page = Page::Browser;
        self.refresh();
        cx.notify()
    }
    fn sort(&mut self, s: MediaSort, cx: &mut Context<Self>) {
        self.state.sort = s;
        self.refresh();
        cx.notify()
    }
    fn import_selected(&mut self, _: &ImportSelected, _: &mut Window, cx: &mut Context<Self>) {
        if self.importing || self.planning {
            return;
        }
        if self.scanning {
            self.message = Some("Wait for the source scan to finish before planning".into());
            cx.notify();
            return;
        }
        if self.plan.is_none() {
            self.start_plan(cx);
            return;
        }
        let Some(source) = self.source.clone() else {
            return;
        };
        let plan = self.plan.clone().expect("plan checked above");
        if plan.items.iter().any(|i| !i.status.can_execute()) {
            self.message = Some("Import preview contains blocked destinations".into());
            cx.notify();
            return;
        }
        self.plan = Some(plan.clone());
        self.last_import_result = None;
        self.deletion_armed = false;
        let cancel = CancellationToken::new();
        let (tx, rx) = mpsc::channel();
        self.work_receivers.push(rx);
        self.import_cancellation = Some(cancel.clone());
        self.importing = true;
        let cat = self.catalog.clone();
        let sid = self.catalog_source_id;
        let media_ids = self.catalog_media_ids.clone();
        let generation = self.state.generation;
        thread::spawn(move || {
            run_import(source, plan, cancel, cat, sid, media_ids, generation, tx)
        });
        cx.notify();
    }
    fn start_plan(&mut self, cx: &mut Context<Self>) {
        let Some(source) = self.source.clone() else {
            self.message = Some("Open a source first".into());
            cx.notify();
            return;
        };
        let inputs: Vec<_> = self
            .state
            .items()
            .filter(|item| self.state.is_selected(item.id))
            .cloned()
            .map(|item| PlanInput {
                capture_time: capture_time(&item),
                item,
            })
            .collect();
        if inputs.is_empty() {
            self.message = Some("Select at least one item".into());
            cx.notify();
            return;
        }
        let preset = self.preset.clone();
        self.plan_revision = self.plan_revision.wrapping_add(1);
        let revision = self.plan_revision;
        let (tx, rx) = mpsc::channel();
        self.work_receivers.push(rx);
        self.planning = true;
        self.message = Some("Building import preview…".into());
        thread::spawn(move || {
            let plan = ImportPlanner::build(source.as_ref(), inputs, &preset);
            let _ = tx.send(WorkMessage::Plan(revision, plan));
        });
        cx.notify();
    }
    fn cancel_import(&mut self, _: &CancelImport, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(t) = &self.import_cancellation {
            t.cancel()
        }
        self.message = Some("Cancelling import…".into());
        cx.notify()
    }
    fn history(&mut self, _: &ShowHistory, _: &mut Window, cx: &mut Context<Self>) {
        let cat = self.catalog.clone();
        let (tx, rx) = mpsc::channel();
        self.work_receivers.push(rx);
        thread::spawn(move || {
            let result = cat.list_sessions(50).and_then(|sessions| {
                sessions
                    .into_iter()
                    .map(|session| {
                        let detail = cat.session_detail(session.id)?;
                        let source = cat.source_by_id(session.source_id)?;
                        let source_name = source
                            .and_then(|source| source.identity.alias.or(source.identity.model))
                            .unwrap_or_else(|| format!("Source {}", session.source_id));
                        Ok(HistoryEntry {
                            detail,
                            source_name,
                        })
                    })
                    .collect::<Result<Vec<_>, captureport_catalog::CatalogError>>()
            });
            match result {
                Ok(entries) => {
                    let _ = tx.send(WorkMessage::History(entries));
                }
                Err(error) => {
                    let _ = tx.send(WorkMessage::Done(Err(error.to_string())));
                }
            }
        });
        cx.notify()
    }
    fn inspect_session(&mut self, id: i64, cx: &mut Context<Self>) {
        self.session_detail = self
            .history
            .iter()
            .find(|entry| entry.detail.session.id == id)
            .map(|entry| entry.detail.clone());
        cx.notify();
    }
    fn show_recovery(&mut self, cx: &mut Context<Self>) {
        self.page = Page::Recovery;
        cx.notify();
    }
    fn resume_from_current_source(&mut self, cx: &mut Context<Self>) {
        let matching = self.catalog_source_id.is_some_and(|id| {
            self.incomplete_sessions
                .iter()
                .any(|session| session.session.source_id == id)
        });
        if !matching || self.scanning {
            self.message = Some(
                "Reconnect the source and finish scanning before building a recovery preview"
                    .into(),
            );
            cx.notify();
            return;
        }
        let ids = self.state.items().map(|item| item.id).collect::<Vec<_>>();
        for id in ids {
            self.state.select(id, false);
        }
        self.state.select_all_new();
        self.invalidate_plan();
        self.start_plan(cx);
    }
    fn delete_sources(&mut self, cx: &mut Context<Self>) {
        let (Some(root), Some(plan), Some(result)) = (
            self.filesystem_root.clone(),
            self.plan.clone(),
            self.last_import_result.clone(),
        ) else {
            self.message =
                Some("A verified filesystem import is required before deleting originals".into());
            cx.notify();
            return;
        };
        if !self.deletion_armed {
            self.deletion_armed = true;
            self.message =
                Some("Review the verified copies, then confirm source deletion separately".into());
            cx.notify();
            return;
        }
        self.deletion_armed = false;
        self.last_import_result = None;
        let (tx, rx) = mpsc::channel();
        self.work_receivers.push(rx);
        thread::spawn(move || {
            let mut remover = VerifiedFilesystemRemover::new(root, &plan);
            let report = captureport_ingest::delete_verified_sources(
                &mut remover,
                &plan.items,
                &result,
                captureport_ingest::DeletionOptions {
                    confirmed: true,
                    require_all_destinations: true,
                },
            );
            let _ = tx.send(WorkMessage::Done(Ok(format!(
                "Deleted {} verified original(s); {} refused; {} failed",
                report.deleted.len(),
                report.refused.len(),
                report.failed.len()
            ))));
        });
        cx.notify();
    }
    fn clean_partial(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(partial) = self.partial_files.get(index).cloned() else {
            return;
        };
        let roots = [
            self.preset.photo.root.clone(),
            self.preset.video.root.clone(),
        ];
        let (tx, rx) = mpsc::channel();
        self.work_receivers.push(rx);
        thread::spawn(move || {
            let result = roots
                .iter()
                .find(|root| partial.path.starts_with(root))
                .map(|root| captureport_ingest::cleanup_partial(root, &partial.path));
            match result {
                Some(Ok(())) => {
                    let _ = tx.send(WorkMessage::PartialCleaned(partial.path));
                }
                Some(Err(error)) => {
                    let _ = tx.send(WorkMessage::Done(Err(error.to_string())));
                }
                None => {
                    let _ = tx.send(WorkMessage::Done(Err(
                        "Partial is outside configured destinations".into(),
                    )));
                }
            }
        });
        cx.notify();
    }
    fn reconcile(&mut self, _: &ReconcileLibrary, _: &mut Window, cx: &mut Context<Self>) {
        if self.importing || self.reconcile_cancellation.is_some() {
            self.message = Some("Finish the active import or reconciliation first".into());
            cx.notify();
            return;
        }
        let roots = vec![
            self.preset.photo.root.clone(),
            self.preset.video.root.clone(),
        ];
        let (tx, rx) = mpsc::channel();
        self.work_receivers.push(rx);
        let catalog = self.catalog.clone();
        let cancel = CancellationToken::new();
        self.reconcile_cancellation = Some(cancel.clone());
        thread::spawn(move || {
            let result = run_reconcile(&roots, &catalog, &cancel);
            let _ = tx.send(WorkMessage::ReconcileDone(result));
        });
        cx.notify()
    }
    fn cancel_reconcile(&mut self, _: &CancelReconcile, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(cancel) = &self.reconcile_cancellation {
            cancel.cancel();
            self.message = Some("Stopping library scan…".into());
            cx.notify();
        }
    }
    fn clock(&mut self, _: &AdjustClock, _: &mut Window, cx: &mut Context<Self>) {
        self.page = Page::Settings;
        cx.notify()
    }
    fn show_settings(&mut self, cx: &mut Context<Self>) {
        self.page = Page::Settings;
        cx.notify();
    }
    fn save_source_alias(&mut self, cx: &mut Context<Self>) {
        let Some(source_id) = self.catalog_source_id else {
            self.message = Some("Open a source before naming it".into());
            cx.notify();
            return;
        };
        let value = self.settings.source_alias.read(cx).value();
        let alias = if value.trim().is_empty() {
            None
        } else {
            Some(value.trim().to_string())
        };
        let catalog = self.catalog.clone();
        let (tx, rx) = mpsc::channel();
        self.work_receivers.push(rx);
        thread::spawn(
            move || match catalog.set_source_alias(source_id, alias.clone()) {
                Ok(_) => {
                    let _ = tx.send(WorkMessage::SourceAlias(alias));
                }
                Err(error) => {
                    let _ = tx.send(WorkMessage::Done(Err(error.to_string())));
                }
            },
        );
        cx.notify();
    }
    fn choose_preset(&mut self, organized: bool, cx: &mut Context<Self>) {
        let replan = self.plan.is_some() || self.planning;
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/"));
        self.preset = if organized {
            ImportPreset::organized(&home)
        } else {
            ImportPreset::everyday(&home)
        };
        self.settings.sync(&self.preset, cx);
        self.backup_required_choice = false;
        self.invalidate_plan();
        self.page = Page::Settings;
        if replan && !self.scanning {
            self.start_plan(cx);
        }
        cx.notify();
    }
    fn cycle_verification(&mut self, cx: &mut Context<Self>) {
        self.preset.verification = match self.preset.verification {
            VerificationMode::Fast => VerificationMode::Standard,
            VerificationMode::Standard => VerificationMode::Strict,
            VerificationMode::Strict => VerificationMode::Fast,
        };
        self.invalidate_plan();
        cx.notify();
    }
    fn cycle_bundle_policy(&mut self, cx: &mut Context<Self>) {
        self.preset.bundle_policy = match self.preset.bundle_policy {
            BundlePolicy::KeepAll => BundlePolicy::RawOnly,
            BundlePolicy::RawOnly => BundlePolicy::JpegOnly,
            BundlePolicy::JpegOnly => BundlePolicy::KeepAll,
        };
        self.invalidate_plan();
        cx.notify();
    }
    fn cycle_grouping(&mut self, cx: &mut Context<Self>) {
        self.preset.grouping = match self.preset.grouping {
            Grouping::None => Grouping::Day,
            Grouping::Day => Grouping::Week,
            Grouping::Week => Grouping::Month,
            Grouping::Month => Grouping::Year,
            Grouping::Year => Grouping::TimeGap {
                threshold_minutes: 30,
            },
            Grouping::TimeGap { .. } => Grouping::None,
        };
        self.invalidate_plan();
        cx.notify();
    }
    fn cycle_collision(&mut self, cx: &mut Context<Self>) {
        self.preset.collision = match self.preset.collision {
            CollisionPolicy::Stop => CollisionPolicy::Skip,
            CollisionPolicy::Skip => CollisionPolicy::UniqueSuffix,
            CollisionPolicy::UniqueSuffix => CollisionPolicy::VerifyIdentical,
            CollisionPolicy::VerifyIdentical => CollisionPolicy::Stop,
        };
        self.invalidate_plan();
        cx.notify();
    }
    fn toggle_backup_required(&mut self, cx: &mut Context<Self>) {
        self.backup_required_choice = !self.backup_required_choice;
        self.invalidate_plan();
        cx.notify();
    }
    fn apply_settings(&mut self, cx: &mut Context<Self>) {
        let preset = match self
            .settings
            .read(&self.preset, self.backup_required_choice, cx)
        {
            Ok(preset) => preset,
            Err(error) => {
                self.message = Some(format!("Settings: {error}"));
                cx.notify();
                return;
            }
        };
        let replan = self.plan.is_some() || self.planning;
        self.preset = preset.clone();
        self.invalidate_plan();
        self.page = Page::Browser;
        self.message = Some("Import settings saved; build a new preview".into());
        let path = self.config_path.clone();
        let (tx, rx) = mpsc::channel();
        self.work_receivers.push(rx);
        thread::spawn(move || {
            let result = (|| -> Result<(), String> {
                let data = serde_json::to_vec_pretty(&preset).map_err(|e| e.to_string())?;
                let temporary = path.with_extension("json.tmp");
                std::fs::write(&temporary, data).map_err(|e| e.to_string())?;
                std::fs::rename(temporary, path).map_err(|e| e.to_string())?;
                Ok(())
            })();
            if let Err(error) = result {
                let _ = tx.send(WorkMessage::Done(Err(error)));
            }
        });
        if replan && !self.scanning {
            self.start_plan(cx);
        }
        cx.notify();
    }
    fn discover_sources(&mut self, _: &DiscoverSources, _: &mut Window, cx: &mut Context<Self>) {
        let (tx, rx) = mpsc::channel();
        self.work_receivers.push(rx);
        let catalog = self.catalog.clone();
        thread::spawn(move || match captureport_gphoto::discovery::discover() {
            Ok(sources) => {
                let aliases = aliases_for_sources(&sources, &catalog);
                let _ = tx.send(WorkMessage::Sources(sources, aliases));
            }
            Err(error) => {
                let _ = tx.send(WorkMessage::Done(Err(error.to_string())));
            }
        });
        cx.notify()
    }
    fn clear_thumbnail_cache(&mut self, cx: &mut Context<Self>) {
        if self.cache_clearing {
            return;
        }
        self.cache_clearing = true;
        let cache = self.cache_dir.join("thumbnails");
        let (tx, rx) = mpsc::channel();
        self.work_receivers.push(rx);
        thread::spawn(move || {
            let result = (|| -> Result<usize, String> {
                let mut removed = 0;
                for entry in std::fs::read_dir(cache).map_err(|e| e.to_string())? {
                    let entry = entry.map_err(|e| e.to_string())?;
                    if entry.path().extension().is_some_and(|ext| ext == "jpg")
                        && entry.file_type().map_err(|e| e.to_string())?.is_file()
                    {
                        std::fs::remove_file(entry.path()).map_err(|e| e.to_string())?;
                        removed += 1;
                    }
                }
                Ok(removed)
            })();
            match result {
                Ok(count) => {
                    let _ = tx.send(WorkMessage::CacheCleared(count));
                }
                Err(error) => {
                    let _ = tx.send(WorkMessage::Done(Err(error)));
                }
            }
        });
        cx.notify();
    }
    fn open_discovered(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(source) = self.discovered_sources.get(index).cloned() else {
            return;
        };
        match source {
            captureport_gphoto::discovery::DiscoveredSource::Ptp(camera) => {
                self.scan(SourceRequest::Camera(camera), cx)
            }
            captureport_gphoto::discovery::DiscoveredSource::MountedFilesystem { path, .. } => {
                self.scan(SourceRequest::Filesystem(path), cx)
            }
        }
    }
}
impl Drop for Browser {
    fn drop(&mut self) {
        self.discovery_stop.store(true, Ordering::Relaxed);
        if let Some(t) = &self.cancellation {
            t.cancel()
        }
        if let Some(t) = &self.import_cancellation {
            t.cancel()
        }
        if let Some(t) = &self.reconcile_cancellation {
            t.cancel();
        }
    }
}
impl Focusable for Browser {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}
include!("render.rs");

fn button(
    label: &'static str,
    handler: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(label)
        .cursor_pointer()
        .rounded_md()
        .px_3()
        .py_2()
        .min_h(px(34.))
        .flex_shrink_0()
        .text_sm()
        .bg(gpui::rgba(0x8a9a8a33))
        .hover(|style| style.bg(gpui::rgba(0x8a9a8a55)))
        .on_click(handler)
        .child(label)
}
fn chip(
    label: &'static str,
    active: bool,
    handler: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let chip = div()
        .id(label)
        .cursor_pointer()
        .rounded_md()
        .px_3()
        .py_1()
        .text_sm()
        .bg(if active {
            rgb(0x247c66)
        } else {
            gpui::rgba(0x8a9a8a22)
        })
        .on_click(handler)
        .child(label);
    if active {
        chip.text_color(rgb(0xffffff))
    } else {
        chip
    }
}
fn capture_time(item: &captureport_core::MediaItem) -> DateTime<FixedOffset> {
    if let captureport_core::MetadataState::Ready(m) = &item.metadata {
        if let Some(t) = &m.capture_time {
            if let Ok(v) = DateTime::parse_from_rfc3339(t) {
                return v;
            }
            if let Ok(v) = NaiveDateTime::parse_from_str(t, "%Y-%m-%d %H:%M:%S") {
                return FixedOffset::east_opt(0)
                    .expect("zero offset")
                    .from_local_datetime(&v)
                    .single()
                    .expect("valid local time");
            }
        }
        if let Some(t) = &m.filesystem_time
            && let Ok(v) = DateTime::parse_from_rfc3339(t)
        {
            return v;
        }
    }
    FixedOffset::east_opt(0)
        .expect("zero offset")
        .timestamp_opt(946684800, 0)
        .single()
        .expect("valid deterministic timestamp")
}
struct VerifiedFilesystemRemover {
    root: PathBuf,
    copies: HashMap<String, Vec<PathBuf>>,
    sizes: HashMap<String, u64>,
}
impl VerifiedFilesystemRemover {
    fn new(root: PathBuf, plan: &ImportPlan) -> Self {
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
struct Recorder {
    catalog: CatalogHandle,
    session_id: i64,
    media_ids: HashMap<MediaId, i64>,
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
fn run_import(
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
        let _ = sender.send(WorkMessage::ImportDone(Err(
            "source was not recorded".into()
        )));
        return;
    };
    let session = match catalog.begin_session(source_id, None, now()) {
        Ok(s) => s,
        Err(e) => {
            let _ = sender.send(WorkMessage::ImportDone(Err(e.to_string())));
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
            },
        };
        let _ = sender.send(WorkMessage::Event(Box::new(event)));
    });
    let (status, message) = match result {
        Ok(result) => {
            let _ = sender.send(WorkMessage::ImportResult(result.clone()));
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
                let completed_required = plan
                    .items
                    .iter()
                    .find(|planned| planned.media_id == item.media_id)
                    .is_some_and(|planned| {
                        planned
                            .copies
                            .iter()
                            .filter(|copy| copy.required)
                            .all(|planned_copy| {
                                item.copies.iter().any(|actual| {
                                    actual.destination == planned_copy.final_destination
                                        && actual.state
                                            == captureport_ingest::ImportItemState::Completed
                                })
                            })
                    });
                if completed_required {
                    let _ = sender.send(WorkMessage::Event(Box::new(
                        AppEvent::ImportStatusChanged {
                            generation,
                            media_id: item.media_id,
                            status: captureport_core::ImportStatus::Imported,
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
    let _ = sender.send(WorkMessage::ImportDone(message));
}
fn run_reconcile(
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
fn aliases_for_sources(
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
fn now() -> String {
    Utc::now().to_rfc3339()
}
fn format_size(bytes: u64) -> String {
    const U: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let (mut v, mut i) = (bytes as f64, 0);
    while v >= 1000. && i < U.len() - 1 {
        v /= 1000.;
        i += 1
    }
    if i == 0 {
        format!("{bytes} B")
    } else {
        format!("{v:.1} {}", U[i])
    }
}
fn main() {
    logging::initialize().expect("could not initialize logging");
    let p = paths::AppPaths::resolve().expect("could not resolve XDG paths");
    p.create().expect("could not create app directories");
    let catalog =
        CatalogHandle::open(p.data.join("catalog.sqlite3")).expect("could not open catalog");
    let cache_dir = p.cache.clone();
    let config_path = p.config.join("preset.json");
    let ui_path = p.config.join("ui.json");
    let mut ui = std::fs::read(&ui_path)
        .ok()
        .and_then(|data| serde_json::from_slice::<UiPreferences>(&data).ok())
        .unwrap_or_default();
    ui.thumbnail_size = ui.thumbnail_size.min(4);
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| p.data.clone());
    let preset = std::fs::read(&config_path)
        .ok()
        .and_then(|data| serde_json::from_slice::<ImportPreset>(&data).ok())
        .unwrap_or_else(|| ImportPreset::everyday(&home));
    let args = std::env::args().collect::<Vec<_>>();
    let initial_source = if args.get(1).is_some_and(|arg| arg == "--demo") {
        let (scenario, count) = match args.get(2).map(String::as_str) {
            Some("empty") => (FakeSourceScenario::NormalCamera, 0),
            Some("camera") => (FakeSourceScenario::NormalCamera, 100),
            Some("importing") => (FakeSourceScenario::SlowCard, 100),
            Some("errors") => (FakeSourceScenario::DisconnectingCamera, 100),
            Some("10000") | None => (FakeSourceScenario::LargeCard, 10_000),
            Some(other) => {
                eprintln!("Unknown demo scenario: {other}");
                std::process::exit(2);
            }
        };
        Some(SourceRequest::Demo(
            FakeSourceBuilder::new().scenario(scenario).files(count),
        ))
    } else {
        None
    };
    let demo_importing = args.get(1).is_some_and(|arg| arg == "--demo")
        && args.get(2).is_some_and(|arg| arg == "importing");
    Application::new().run(move |cx: &mut App| {
        cx.bind_keys([
            KeyBinding::new("cmd-o", OpenFolder, None),
            KeyBinding::new("cmd-d", OpenDemo, None),
            KeyBinding::new("cmd-a", SelectAll, None),
            KeyBinding::new("cmd-shift-a", SelectAllNew, None),
            KeyBinding::new("escape", SelectNone, None),
            KeyBinding::new("cmd-i", ImportSelected, None),
            KeyBinding::new(
                "backspace",
                text_input::Backspace,
                Some("CapturePortTextInput"),
            ),
            KeyBinding::new("delete", text_input::Delete, Some("CapturePortTextInput")),
            KeyBinding::new("left", text_input::Left, Some("CapturePortTextInput")),
            KeyBinding::new("right", text_input::Right, Some("CapturePortTextInput")),
            KeyBinding::new("home", text_input::Home, Some("CapturePortTextInput")),
            KeyBinding::new("end", text_input::End, Some("CapturePortTextInput")),
            KeyBinding::new("cmd-a", text_input::SelectAll, Some("CapturePortTextInput")),
        ]);
        let bounds = Bounds::centered(None, size(px(1180.), px(760.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |window, cx| {
                cx.new(|cx| {
                    Browser::new(
                        window,
                        cx,
                        Startup {
                            catalog,
                            cache_dir,
                            config_path,
                            ui_path,
                            ui,
                            preset,
                            initial_source,
                            demo_importing,
                        },
                    )
                })
            },
        )
        .expect("could not open CapturePort window");
        cx.activate(true)
    })
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use captureport_ingest::PlanStatus;
    use std::fs;

    #[test]
    fn filesystem_scan_plan_import_and_repeat_scan_match_history() {
        let source_dir = tempfile::tempdir().unwrap();
        let destination_dir = tempfile::tempdir().unwrap();
        fs::write(
            source_dir.path().join("IMG0001.JPG"),
            b"captureport test photo",
        )
        .unwrap();
        let catalog = CatalogHandle::open(captureport_catalog::CatalogPath::Memory).unwrap();
        let (tx, rx) = mpsc::channel();
        scan_source(
            SourceRequest::Filesystem(source_dir.path().to_path_buf()),
            ScanContext::new(ScanGeneration(1), CancellationToken::new()),
            tx,
            catalog.clone(),
        );
        let mut state = AppState::new();
        let mut source = None;
        let mut catalog_ids = HashMap::new();
        for message in rx {
            match message {
                ScanMessage::Source(found, _, _) => source = Some(found),
                ScanMessage::CatalogMedia(id, catalog_id) => {
                    catalog_ids.insert(id, catalog_id);
                }
                ScanMessage::Event(event) => {
                    state.apply_event(*event);
                }
                ScanMessage::Finished(result) => {
                    result.unwrap();
                    break;
                }
                ScanMessage::ThumbnailInfo(..) => {}
            }
        }
        assert_eq!(state.len(), 1);
        let source = source.unwrap();
        let mut preset = ImportPreset::everyday(destination_dir.path());
        preset.photo.root = destination_dir.path().to_path_buf();
        preset.photo.folder_template.clear();
        let inputs = state
            .items()
            .cloned()
            .map(|item| PlanInput {
                capture_time: capture_time(&item),
                item,
            })
            .collect();
        let plan = ImportPlanner::build(source.as_ref(), inputs, &preset);
        assert_eq!(plan.items[0].status, PlanStatus::Ready);
        let destination = plan.items[0].copies[0].final_destination.clone();
        let (tx, rx) = mpsc::channel();
        run_import(
            source,
            plan.clone(),
            CancellationToken::new(),
            catalog.clone(),
            Some(1),
            catalog_ids,
            ScanGeneration(1),
            tx,
        );
        let mut result = None;
        for message in rx {
            match message {
                WorkMessage::ImportResult(imported) => result = Some(imported),
                WorkMessage::ImportDone(done) => {
                    done.unwrap();
                    break;
                }
                _ => {}
            }
        }
        let result = result.unwrap();
        assert_eq!(
            result.items[0].copies[0].state,
            captureport_ingest::ImportItemState::Completed
        );
        assert_eq!(fs::read(&destination).unwrap(), b"captureport test photo");
        assert_eq!(
            fs::read(source_dir.path().join("IMG0001.JPG")).unwrap(),
            b"captureport test photo"
        );
        assert_eq!(
            catalog.list_sessions(10).unwrap()[0].status,
            captureport_catalog::SessionStatus::Completed
        );

        let (tx, rx) = mpsc::channel();
        scan_source(
            SourceRequest::Filesystem(source_dir.path().to_path_buf()),
            ScanContext::new(ScanGeneration(2), CancellationToken::new()),
            tx,
            catalog,
        );
        let imported=rx.into_iter().any(|message|matches!(message,ScanMessage::Event(event)
            if matches!(*event,AppEvent::ImportStatusChanged {status:captureport_core::ImportStatus::Imported,..})));
        assert!(imported);
        let mut remover = VerifiedFilesystemRemover::new(source_dir.path().to_path_buf(), &plan);
        let deleted = captureport_ingest::delete_verified_sources(
            &mut remover,
            &plan.items,
            &result,
            captureport_ingest::DeletionOptions {
                confirmed: true,
                require_all_destinations: true,
            },
        );
        assert_eq!(deleted.deleted.len(), 1);
        assert!(!source_dir.path().join("IMG0001.JPG").exists());
        assert_eq!(fs::read(destination).unwrap(), b"captureport test photo");
    }
}

mod browser;
mod devices;
mod icons;
mod logging;
mod model;
mod paths;
mod saved_presets;
mod scan;
mod scrollbar;
mod spacing;
mod tasks;
mod template_help;
mod text_input;
mod widgets;
pub(crate) use devices::*;
pub(crate) use model::*;
pub(crate) use tasks::*;
pub(crate) use widgets::*;

use captureport_catalog::{
    CatalogHandle, ImportInput, ImportStatus as CatalogImportStatus, IncompleteSession,
    ReconciledImportInput, SessionDetail,
};
use captureport_core::{
    AppEvent, AppState, CancellationToken, FakeSourceBuilder, FakeSourceScenario, MediaFilter,
    MediaId, MediaLocator, MediaSort, MediaSource, MediaType, ScanContext, ScanGeneration,
    SourceError, SourceId, SourceType,
};
use captureport_ingest::{
    BackupRule, BundlePolicy, BundleType, CollisionPolicy, CopyDigest, DestinationRule, Grouping,
    ImportEngine, ImportPlan, ImportPlanner, ImportPreset, ImportRecorder, PlanInput, PlanStatus,
    PlannedCopy, PlannedImport, VerificationMode, effective_time, session_numbers,
};
use captureport_metadata::{ThumbnailPipeline, ThumbnailRequest, ThumbnailState};
use chrono::{DateTime, FixedOffset, Local, NaiveDateTime, TimeZone, Utc};
use gpui::{
    App, Application, Bounds, Context, Entity, FocusHandle, Focusable, KeyBinding,
    PathPromptOptions, Timer, Window, WindowBounds, WindowOptions, actions, div, img, prelude::*,
    px, rgb, size, uniform_list,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    process::Command,
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
        MarkSelectedImported,
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

struct Browser {
    scrollbars: scrollbar::Scrollbars,
    state: AppState,
    visible_ids: Vec<MediaId>,
    gallery_groups: Vec<GalleryGroup>,
    gallery_names: HashMap<String, String>,
    /// Session display title per item, so `{session_name}` survives filters.
    gallery_item_names: HashMap<MediaId, String>,
    gallery_names_path: PathBuf,
    gallery_edit_key: Option<String>,
    gallery_edit_input: Entity<text_input::TextInput>,
    bundles: HashMap<MediaId, Vec<MediaId>>,
    bundle_members: HashSet<MediaId>,
    bundle_owner: HashMap<MediaId, MediaId>,
    /// Display id -> member whose thumbnail represents the bundle (a RAW+JPEG
    /// pair renders the JPEG). Absent means the item uses its own thumbnail.
    bundle_preview: HashMap<MediaId, MediaId>,
    pair_index: PairIndex,
    expanded_bundle: Option<MediaId>,
    media_detail: Option<MediaId>,
    detail_focus: FocusHandle,
    show_view_options: bool,
    timezone_menu_open: bool,
    explicit_bundle_selection: HashSet<MediaId>,
    receiver: Option<Receiver<ScanMessage>>,
    work_receivers: Vec<Receiver<WorkMessage>>,
    cancellation: Option<CancellationToken>,
    import_cancellation: Option<CancellationToken>,
    reconcile_cancellation: Option<CancellationToken>,
    last_import_result: Option<captureport_ingest::ImportResult>,
    deletion_armed: bool,
    /// Which incomplete file's destructive control has been armed for confirmation.
    armed_partial: Option<usize>,
    source: Option<Arc<dyn MediaSource>>,
    filesystem_root: Option<PathBuf>,
    catalog: CatalogHandle,
    catalog_source_id: Option<i64>,
    source_alias: Option<String>,
    catalog_media_ids: HashMap<MediaId, i64>,
    preset: ImportPreset,
    backup_required_choice: bool,
    settings: SettingsInputs,
    saved_presets: Vec<captureport_catalog::PresetRecord>,
    selected_preset: Option<i64>,
    preset_busy: bool,
    preset_confirmation: Option<saved_presets::PresetAction>,
    preset_message: Option<String>,
    template_segment_target: Option<Entity<text_input::TextInput>>,
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
    discovery_receiver: Receiver<(bool, DiscoveryResult, RemovableVolumes)>,
    discovery_requests: mpsc::Sender<()>,
    discovering: bool,
    unmounted_cards: Vec<UnmountedCard>,
    /// Device model keyed by mount point, for detected removable sources.
    mounted_models: HashMap<PathBuf, String>,
    mounting: Option<PathBuf>,
    discovery_stop: Arc<AtomicBool>,
    incomplete_sessions: Vec<IncompleteSession>,
    partial_files: Vec<captureport_ingest::PartialFile>,
    thumbnails: ThumbnailPipeline,
    thumbnail_keys: HashMap<String, MediaId>,
    thumbnail_paths: HashMap<MediaId, PathBuf>,
    thumbnail_modified: HashMap<MediaId, u64>,
    requested_thumbnails: HashSet<MediaId>,
    failed_thumbnails: HashSet<MediaId>,
    camera_preview_sender: SyncSender<CameraPreviewJob>,
    camera_preview_receiver: Receiver<CameraPreviewResult>,
    camera_preview_scan_token: u128,
    cache_clearing: bool,
    cache_dir: PathBuf,
    generation: u64,
    scanning: bool,
    importing: bool,
    marking_imported: bool,
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
            gallery_names_path,
            ui,
            preset,
            initial_source,
            demo_importing,
        } = startup;
        let focus = cx.focus_handle();
        window.focus(&focus);
        let settings = SettingsInputs::new(&preset, cx);
        for input in [
            &settings.photo_folder,
            &settings.video_folder,
            &settings.filename,
        ] {
            cx.observe(input, |_, _, cx| cx.notify()).detach();
        }
        let gallery_names = std::fs::read(&gallery_names_path)
            .ok()
            .and_then(|data| serde_json::from_slice(&data).ok())
            .unwrap_or_default();
        let gallery_edit_input =
            cx.new(|cx| text_input::TextInput::new(String::new(), "Gallery name", cx));
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
        let (request_tx, request_rx) = mpsc::channel();
        let discovery_catalog = catalog.clone();
        let discovery_stop = Arc::new(AtomicBool::new(false));
        let discovery_worker_stop = discovery_stop.clone();
        thread::spawn(move || {
            let mut manual = false;
            while !discovery_worker_stop.load(Ordering::Relaxed) {
                let result = captureport_gphoto::discovery::discover()
                    .map(|sources| {
                        let aliases = aliases_for_sources(&sources, &discovery_catalog);
                        (sources, aliases)
                    })
                    .map_err(|error| error.to_string());
                if discovery_tx
                    .send((manual, result, removable_volumes().unwrap_or_default()))
                    .is_err()
                {
                    break;
                }
                manual = request_rx.recv_timeout(Duration::from_secs(3)).is_ok();
            }
        });
        let presets_catalog = catalog.clone();
        let (presets_tx, presets_rx) = mpsc::channel();
        thread::spawn(move || {
            let result = presets_catalog
                .list_presets()
                .map(|presets| {
                    (
                        presets,
                        None,
                        "Choose a preset to load its settings.".into(),
                    )
                })
                .map_err(|error| error.to_string());
            let _ = presets_tx.send(WorkMessage::Presets(result));
        });
        let mut browser = Self {
            scrollbars: scrollbar::Scrollbars::default(),
            state: AppState::new(),
            visible_ids: Vec::new(),
            gallery_groups: Vec::new(),
            gallery_names,
            gallery_item_names: HashMap::new(),
            gallery_names_path,
            gallery_edit_key: None,
            gallery_edit_input,
            bundles: HashMap::new(),
            bundle_members: HashSet::new(),
            bundle_owner: HashMap::new(),
            bundle_preview: HashMap::new(),
            pair_index: PairIndex::default(),
            expanded_bundle: None,
            media_detail: None,
            detail_focus: cx.focus_handle(),
            show_view_options: false,
            timezone_menu_open: false,
            explicit_bundle_selection: HashSet::new(),
            receiver: None,
            work_receivers: vec![recovery_rx, presets_rx],
            cancellation: None,
            import_cancellation: None,
            reconcile_cancellation: None,
            last_import_result: None,
            deletion_armed: false,
            armed_partial: None,
            source: None,
            filesystem_root: None,
            catalog,
            catalog_source_id: None,
            source_alias: None,
            catalog_media_ids: HashMap::new(),
            preset,
            backup_required_choice,
            settings,
            saved_presets: Vec::new(),
            selected_preset: None,
            preset_busy: true,
            preset_confirmation: None,
            preset_message: None,
            template_segment_target: None,
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
            discovery_requests: request_tx,
            discovering: false,
            unmounted_cards: Vec::new(),
            mounted_models: HashMap::new(),
            mounting: None,
            discovery_stop,
            incomplete_sessions: Vec::new(),
            partial_files: Vec::new(),
            thumbnails: ThumbnailPipeline::new(cache_dir.join("thumbnails"), 2, 128)
                .expect("thumbnail cache"),
            thumbnail_keys: HashMap::new(),
            thumbnail_paths: HashMap::new(),
            thumbnail_modified: HashMap::new(),
            requested_thumbnails: HashSet::new(),
            failed_thumbnails: HashSet::new(),
            camera_preview_sender,
            camera_preview_receiver,
            camera_preview_scan_token: 0,
            cache_clearing: false,
            cache_dir,
            generation: 0,
            scanning: false,
            importing: false,
            marking_imported: false,
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

fn main() {
    logging::initialize().expect("could not initialize logging");
    let p = paths::AppPaths::resolve().expect("could not resolve XDG paths");
    p.create().expect("could not create app directories");
    let args = std::env::args().collect::<Vec<_>>();
    let demo = args.get(1).is_some_and(|arg| arg == "--demo");
    // A fixture run must not write synthetic rows into the user's real catalog:
    // those rows would later read back as possible duplicates of real media.
    let catalog = if demo {
        CatalogHandle::open(captureport_catalog::CatalogPath::Memory)
            .expect("could not open demo catalog")
    } else {
        CatalogHandle::open(p.data.join("catalog.sqlite3")).expect("could not open catalog")
    };
    let cache_dir = p.cache.clone();
    let config_path = p.config.join("preset.json");
    let ui_path = p.config.join("ui.json");
    let gallery_names_path = p.config.join("gallery_names.json");
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
    let initial_source = if demo {
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
    Application::new()
        .with_assets(icons::Icons)
        .run(move |cx: &mut App| {
            if let Err(error) = cx.text_system().add_fonts(vec![
                std::borrow::Cow::Borrowed(include_bytes!("../assets/fonts/Outfit-Regular.ttf")),
                std::borrow::Cow::Borrowed(include_bytes!("../assets/fonts/Outfit-SemiBold.ttf")),
                std::borrow::Cow::Borrowed(include_bytes!("../assets/fonts/Spectral-SemiBold.ttf")),
                std::borrow::Cow::Borrowed(include_bytes!("../assets/fonts/Spectral-Bold.ttf")),
            ]) {
                tracing::warn!(%error, "Could not load bundled fonts");
            }
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
                KeyBinding::new("cmd-v", text_input::Paste, Some("CapturePortTextInput")),
                KeyBinding::new("cmd-c", text_input::Copy, Some("CapturePortTextInput")),
                KeyBinding::new("cmd-x", text_input::Cut, Some("CapturePortTextInput")),
            ]);
            let bounds = Bounds::centered(None, size(px(1180.), px(760.)), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    // Match `captureport.desktop` so the compositor shows the
                    // installed hicolor icon on the window and dock entry.
                    app_id: Some("captureport".into()),
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
                                gallery_names_path,
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
    #[test]
    fn separate_mode_does_not_merge_raw_jpeg_pairs() {
        let pair = captureport_ingest::MediaBundle {
            id: captureport_core::BundleId(1),
            primary: MediaId(1),
            members: vec![MediaId(1), MediaId(2)],
            bundle_type: BundleType::RawJpeg,
        };
        let video = captureport_ingest::MediaBundle {
            bundle_type: BundleType::VideoSidecar,
            ..pair.clone()
        };
        let single = captureport_ingest::MediaBundle {
            members: vec![MediaId(3)],
            ..pair.clone()
        };
        assert!(shows_as_bundle(&pair, true));
        assert!(!shows_as_bundle(&pair, false));
        // Only RAW+JPEG pairs follow the toggle; video sidecars stay merged.
        assert!(shows_as_bundle(&video, false));
        assert!(!shows_as_bundle(&single, true));
    }

    #[test]
    fn bundled_fonts_are_true_type_data() {
        // Guards against a bad download or license file sneaking into an
        // `include_bytes!` slot: GPUI only logs a warning when a face fails to
        // parse, so a wrong file would silently fall back to a system font.
        for (name, bytes) in [
            (
                "Outfit-Regular",
                include_bytes!("../assets/fonts/Outfit-Regular.ttf").as_slice(),
            ),
            (
                "Outfit-SemiBold",
                include_bytes!("../assets/fonts/Outfit-SemiBold.ttf").as_slice(),
            ),
            (
                "Spectral-SemiBold",
                include_bytes!("../assets/fonts/Spectral-SemiBold.ttf").as_slice(),
            ),
            (
                "Spectral-Bold",
                include_bytes!("../assets/fonts/Spectral-Bold.ttf").as_slice(),
            ),
        ] {
            assert!(bytes.len() > 1000, "{name} is too small to be a font");
            let magic = u32::from_be_bytes(bytes[..4].try_into().unwrap());
            let truetype = 0x0001_0000;
            let opentype = u32::from_be_bytes(*b"OTTO");
            assert!(
                magic == truetype || magic == opentype,
                "{name} is not TrueType or OpenType (magic {magic:#010x})"
            );
        }
    }

    #[test]
    fn skipped_collisions_do_not_block_other_imports() {
        assert!(!blocks_import(PlanStatus::Skipped));
        assert!(!blocks_import(PlanStatus::Ready));
        assert!(blocks_import(PlanStatus::DestinationCollision));
    }
    #[test]
    fn pair_index_matches_jpeg_siblings_in_either_order() {
        let mut index = PairIndex::default();
        index.note(MediaId(1), MediaType::Raw, "DCIM/DJI_1.DNG");
        index.note(MediaId(2), MediaType::Jpeg, "DCIM/DJI_1.JPG");
        assert!(index.has_pair(MediaId(1)));
        assert!(!index.has_pair(MediaId(2)));
        assert_eq!(index.paired.get(&MediaId(1)), Some(&MediaId(2)));

        // Discovery order must not matter.
        let mut reverse = PairIndex::default();
        reverse.note(MediaId(3), MediaType::Jpeg, "DCIM/DJI_2.JPG");
        reverse.note(MediaId(4), MediaType::Raw, "DCIM/DJI_2.DNG");
        assert!(reverse.has_pair(MediaId(4)));
        assert_eq!(reverse.paired.get(&MediaId(4)), Some(&MediaId(3)));

        // A different directory is not a pair.
        let mut other = PairIndex::default();
        other.note(MediaId(5), MediaType::Raw, "a/DJI_3.DNG");
        other.note(MediaId(6), MediaType::Jpeg, "b/DJI_3.JPG");
        assert!(!other.has_pair(MediaId(5)));

        // The authoritative bundle pass replaces the streaming guess.
        index.replace([(MediaId(7), MediaId(8))]);
        assert!(!index.has_pair(MediaId(1)));
        assert!(index.has_pair(MediaId(7)));
    }
    /// Guards the demo fixtures' whole purpose: they must be able to fill the
    /// browser grid. A fixture whose preview cannot be decoded makes every demo
    /// state render as an empty grid, which is how this test came to exist.
    #[test]
    fn demo_source_previews_decode_through_the_thumbnail_encoder() {
        use captureport_core::{FakeSourceScenario, MediaSource, ScanContext};
        for scenario in [
            FakeSourceScenario::NormalCamera,
            FakeSourceScenario::LargeCard,
            FakeSourceScenario::SlowCard,
            FakeSourceScenario::DisconnectingCamera,
        ] {
            let source = FakeSourceBuilder::new().scenario(scenario).files(4).build();
            let scan = ScanContext::new(
                captureport_core::ScanGeneration(1),
                captureport_core::CancellationToken::new(),
            );
            let mut items = Vec::new();
            // The disconnecting scenario ends enumeration early by design; the
            // items it did emit still need usable previews.
            let _ = source.enumerate(&scan, &mut |item| {
                items.push(item);
                Ok(())
            });
            assert!(!items.is_empty(), "{scenario:?} emitted no items");
            for item in &items {
                let bytes = source
                    .preview(&item.locator)
                    .unwrap_or_else(|e| panic!("{scenario:?} preview failed: {e}"))
                    .unwrap_or_else(|| panic!("{scenario:?} returned no preview"));
                let decoded = image::load_from_memory(&bytes)
                    .unwrap_or_else(|e| panic!("{scenario:?} preview undecodable: {e}"));
                assert!(decoded.width() > 0 && decoded.height() > 0);
            }
        }
    }

    #[test]
    fn ready_tiles_do_not_jump_when_earlier_media_finishes() {
        let mut displayed = vec![MediaId(2)];
        append_ready_ids(&mut displayed, vec![MediaId(1), MediaId(2)]);
        assert_eq!(displayed, vec![MediaId(2), MediaId(1)]);
        append_ready_ids(&mut displayed, vec![MediaId(1)]);
        assert_eq!(displayed, vec![MediaId(1)]);
    }
    #[test]
    fn only_unmounted_removable_filesystems_are_mountable() {
        let blocks: BlockListing = serde_json::from_str(r#"{"blockdevices":[
            {"path":"/dev/sda","type":"disk","rm":false,"tran":"sata","fstype":null,"mountpoint":null,"label":null,"model":null,"serial":"internal"},
            {"path":"/dev/sda1","type":"part","rm":false,"tran":null,"fstype":"ext4","mountpoint":null,"label":null,"model":null,"serial":null},
            {"path":"/dev/sdb","type":"disk","rm":true,"tran":"usb","fstype":null,"mountpoint":null,"label":null,"model":"Camera","serial":"123"},
            {"path":"/dev/sdb1","type":"part","rm":true,"tran":null,"fstype":"exfat","mountpoint":null,"label":null,"model":null,"serial":null},
            {"path":"/dev/sdb2","type":"part","rm":true,"tran":null,"fstype":"exfat","mountpoint":"/run/media/card","label":null,"model":null,"serial":null}
        ]}"#).unwrap();
        let cards = cards_from_blocks(&blocks.blockdevices);
        assert_eq!(cards.unmounted.len(), 1);
        assert_eq!(cards.unmounted[0].path, PathBuf::from("/dev/sdb1"));
        assert_eq!(cards.unmounted[0].label, "Camera");
        assert_eq!(
            cards.mounted_models.get(&PathBuf::from("/run/media/card")),
            Some(&"Camera".to_string())
        );
    }
    use std::fs;

    #[test]
    fn selected_manual_import_marks_survive_scan_and_detect_changed_content() {
        let source_dir = tempfile::tempdir().unwrap();
        for (name, bytes) in [
            ("PAIR.ARW", b"manual raw frame".as_slice()),
            ("PAIR.JPG", b"manual jpeg frame".as_slice()),
            ("KEEP.JPG", b"unmarked frame".as_slice()),
        ] {
            fs::write(source_dir.path().join(name), bytes).unwrap();
        }
        let catalog = CatalogHandle::open(captureport_catalog::CatalogPath::Memory).unwrap();
        let scan = |generation| {
            let (tx, rx) = mpsc::channel();
            scan_source(
                SourceRequest::Filesystem(source_dir.path().into()),
                ScanContext::new(ScanGeneration(generation), CancellationToken::new()),
                captureport_ingest::MediaRules::default(),
                tx,
                catalog.clone(),
            );
            let mut state = AppState::new();
            let mut catalog_ids = HashMap::new();
            for message in rx {
                match message {
                    ScanMessage::Event(event) => {
                        state.apply_event(*event);
                    }
                    ScanMessage::CatalogMedia(id, catalog_id) => {
                        catalog_ids.insert(id, catalog_id);
                    }
                    ScanMessage::Finished(result) => {
                        result.unwrap();
                        break;
                    }
                    _ => {}
                }
            }
            (state, catalog_ids)
        };
        let (mut state, catalog_ids) = scan(1);
        let ids: Vec<_> = state
            .items()
            .map(|item| (item.id, item.source_name.clone()))
            .collect();
        for (id, name) in &ids {
            state.select(*id, name.starts_with("PAIR."));
        }
        // The action uses selected files even when the current filter hides them.
        state.filter = MediaFilter::Videos;
        assert!(state.visible_items().is_empty());
        let selected = manual_mark_selection(&state, &catalog_ids).unwrap();
        assert_eq!(selected.len(), 2);
        // Missing one catalog row must prevent a partial optimistic update.
        let mut incomplete_ids = catalog_ids.clone();
        incomplete_ids.remove(&selected[0].0);
        assert!(manual_mark_selection(&state, &incomplete_ids).is_err());
        assert_eq!(state.selected_count(), 2);
        catalog
            .mark_manually_imported(selected.iter().map(|(_, id)| *id).collect(), now())
            .unwrap();
        for (id, _) in selected {
            state.apply_event(AppEvent::ManualImportMarked {
                generation: ScanGeneration(1),
                media_id: id,
            });
            assert_eq!(
                import_status_line(state.item(id).unwrap()),
                "✓ Imported · marked manually"
            );
        }
        assert_eq!(state.selected_count(), 0);
        assert!(catalog.list_sessions(10).unwrap().is_empty());
        assert_eq!(
            fs::read(source_dir.path().join("PAIR.JPG")).unwrap(),
            b"manual jpeg frame"
        );
        assert_eq!(fs::read_dir(source_dir.path()).unwrap().count(), 3);

        fs::write(source_dir.path().join("NEW.JPG"), b"new frame").unwrap();
        let (mut restored, _) = scan(2);
        restored.filter = MediaFilter::Imported;
        assert_eq!(restored.visible_items().len(), 2);
        assert!(
            restored
                .visible_items()
                .iter()
                .all(|item| item.manually_marked_imported)
        );
        restored.filter = MediaFilter::All;
        restored.select_all_new();
        assert_eq!(restored.selected_count(), 1); // Only NEW.JPG was auto-selected.
        assert!(
            restored
                .items()
                .filter(|item| item.manually_marked_imported)
                .all(|item| !restored.is_selected(item.id))
        );

        // The same path and size with changed contents must not inherit the declaration.
        let mut changed = b"manual jpeg frame".to_vec();
        *changed.last_mut().unwrap() = b'X';
        fs::write(source_dir.path().join("PAIR.JPG"), changed).unwrap();
        let (rescanned, _) = scan(3);
        let jpeg = rescanned
            .items()
            .find(|item| item.source_name == "PAIR.JPG")
            .unwrap();
        assert!(!jpeg.manually_marked_imported);
        assert_ne!(jpeg.import_status, captureport_core::ImportStatus::Imported);
        let raw = rescanned
            .items()
            .find(|item| item.source_name == "PAIR.ARW")
            .unwrap();
        assert!(raw.manually_marked_imported);
    }

    #[test]
    fn modification_time_is_displayed_in_the_selected_timezone() {
        // 2024-05-01T10:00:00Z
        let epoch = 1_714_557_600;
        assert_eq!(format_file_time(epoch, Some(0)), "2024-05-01 10:00");
        assert_eq!(format_file_time(epoch, Some(2 * 3600)), "2024-05-01 12:00");
        assert_eq!(format_file_time(epoch, Some(-5 * 3600)), "2024-05-01 05:00");
    }

    #[test]
    fn duplicate_gallery_dates_get_letter_suffixes() {
        let group = |key: &str, title: &str| GalleryGroup {
            key: key.into(),
            title: title.into(),
            session: 1,
            ids: Vec::new(),
        };
        let mut groups = vec![
            group("a", "2026-09-18"),
            group("b", "2026-09-18"),
            group("c", "2026-09-19"),
            group("d", "2026-09-18"),
        ];
        suffix_duplicate_titles(&mut groups, |key| key == "d");
        let titles: Vec<_> = groups.iter().map(|group| group.title.as_str()).collect();
        assert_eq!(
            titles,
            ["2026-09-18a", "2026-09-18b", "2026-09-19", "2026-09-18"]
        );
    }

    #[test]
    fn legacy_session_names_are_treated_as_defaults() {
        assert!(is_legacy_session_name("Session 2"));
        assert!(is_legacy_session_name("Session 12"));
        assert!(!is_legacy_session_name("Session"));
        assert!(!is_legacy_session_name("Session a"));
        assert!(!is_legacy_session_name("2026-09-18"));
        assert!(!is_legacy_session_name("My trip"));
    }

    #[test]
    fn a_group_remains_visible_when_only_its_child_matches_a_filter() {
        let owners = HashMap::from([(MediaId(2), MediaId(1))]);
        assert_eq!(visible_capture_ids([MediaId(2)], &owners), vec![MediaId(1)]);
        assert_eq!(
            visible_capture_ids([MediaId(2), MediaId(1), MediaId(3)], &owners),
            vec![MediaId(1), MediaId(3)]
        );
    }

    #[test]
    fn scan_media_rules_drop_non_media_and_allow_odd_extensions() {
        let source_dir = tempfile::tempdir().unwrap();
        fs::write(source_dir.path().join("SHOT.JPG"), b"photo").unwrap();
        fs::write(source_dir.path().join("NOTES.TXT"), b"notes").unwrap();
        let catalog = CatalogHandle::open(captureport_catalog::CatalogPath::Memory).unwrap();
        let scan_names = |rules: captureport_ingest::MediaRules, catalog: CatalogHandle| {
            let (tx, rx) = mpsc::channel();
            scan_source(
                SourceRequest::Filesystem(source_dir.path().to_path_buf()),
                ScanContext::new(ScanGeneration(1), CancellationToken::new()),
                rules,
                tx,
                catalog,
            );
            let mut state = AppState::new();
            for message in rx {
                match message {
                    ScanMessage::Event(event) => {
                        state.apply_event(*event);
                    }
                    ScanMessage::Finished(result) => {
                        result.unwrap();
                        break;
                    }
                    _ => {}
                }
            }
            let mut names: Vec<_> = state.items().map(|item| item.source_name.clone()).collect();
            names.sort();
            names
        };
        assert_eq!(
            scan_names(captureport_ingest::MediaRules::default(), catalog.clone()),
            vec!["SHOT.JPG"]
        );
        let include_txt = captureport_ingest::MediaRules {
            include_photo: vec!["TXT".into()],
            ..Default::default()
        };
        assert_eq!(
            scan_names(include_txt, catalog.clone()),
            vec!["NOTES.TXT", "SHOT.JPG"]
        );
        fs::create_dir_all(source_dir.path().join("Private/nested")).unwrap();
        fs::write(
            source_dir.path().join("Private/nested/HIDDEN.JPG"),
            b"hidden",
        )
        .unwrap();
        fs::create_dir(source_dir.path().join("Private2")).unwrap();
        fs::write(source_dir.path().join("Private2/KEPT.JPG"), b"kept").unwrap();
        let exclude_folder = captureport_ingest::MediaRules {
            exclude_folders: vec!["Private".into()],
            ..Default::default()
        };
        assert_eq!(
            scan_names(exclude_folder, catalog),
            vec!["KEPT.JPG", "SHOT.JPG"]
        );
    }

    #[test]
    fn individual_and_whole_bundle_selection_control_plan_and_history() {
        let source_dir = tempfile::tempdir().unwrap();
        let destination_dir = tempfile::tempdir().unwrap();
        fs::write(source_dir.path().join("PAIR.ARW"), b"raw capture").unwrap();
        fs::write(source_dir.path().join("PAIR.JPG"), b"jpeg capture").unwrap();
        let catalog = CatalogHandle::open(captureport_catalog::CatalogPath::Memory).unwrap();
        let (tx, rx) = mpsc::channel();
        scan_source(
            SourceRequest::Filesystem(source_dir.path().to_path_buf()),
            ScanContext::new(ScanGeneration(1), CancellationToken::new()),
            captureport_ingest::MediaRules::default(),
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
        let source = source.unwrap();
        let jpeg = state
            .items()
            .find(|item| item.source_name == "PAIR.JPG")
            .unwrap()
            .id;
        let inputs = state
            .items()
            .filter(|item| item.id == jpeg)
            .cloned()
            .map(|item| PlanInput {
                capture_time: capture_time(&item),
                session_name: None,
                item,
            })
            .collect();
        let mut preset = ImportPreset::everyday(destination_dir.path());
        preset.photo.root = destination_dir.path().to_path_buf();
        preset.photo.folder_template.clear();
        preset.bundle_policy = BundlePolicy::RawOnly;
        let plan = ImportPlanner::build(source.as_ref(), inputs, &preset);
        assert_eq!(plan.items.len(), 1);
        assert_eq!(plan.items[0].media_id, jpeg);
        let (tx, rx) = mpsc::channel();
        run_import(
            source.clone(),
            plan,
            CancellationToken::new(),
            catalog.clone(),
            Some(1),
            catalog_ids.clone(),
            ScanGeneration(1),
            tx,
        );
        for message in rx {
            if let WorkMessage::ImportDone(result) = message {
                result.unwrap();
                break;
            }
        }
        let session = catalog.list_sessions(10).unwrap().remove(0);
        let history = catalog.session_detail(session.id).unwrap();
        assert_eq!(history.imports.len(), 1);
        assert_eq!(history.imports[0].media_id, catalog_ids[&jpeg]);

        let second_destination = tempfile::tempdir().unwrap();
        preset.photo.root = second_destination.path().to_path_buf();
        let selected = state
            .items()
            .cloned()
            .map(|item| PlanInput {
                capture_time: capture_time(&item),
                session_name: None,
                item,
            })
            .collect();
        let explicit_members = state.items().map(|item| item.id).collect();
        let plan = ImportPlanner::build_with_explicit_members(
            source.as_ref(),
            selected,
            &preset,
            &explicit_members,
        );
        assert_eq!(plan.items.len(), 2);
        let (tx, rx) = mpsc::channel();
        run_import(
            source,
            plan,
            CancellationToken::new(),
            catalog.clone(),
            Some(1),
            catalog_ids.clone(),
            ScanGeneration(1),
            tx,
        );
        for message in rx {
            if let WorkMessage::ImportDone(result) = message {
                result.unwrap();
                break;
            }
        }
        let latest_session = catalog.list_sessions(10).unwrap().remove(0);
        let history = catalog.session_detail(latest_session.id).unwrap();
        let recorded = history
            .imports
            .iter()
            .map(|item| item.media_id)
            .collect::<HashSet<_>>();
        assert_eq!(recorded.len(), 2);
        assert_eq!(recorded, catalog_ids.values().copied().collect());
    }

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
            captureport_ingest::MediaRules::default(),
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
                session_name: None,
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
        let recorded_session = catalog.list_sessions(10).unwrap()[0].id;

        let (tx, rx) = mpsc::channel();
        scan_source(
            SourceRequest::Filesystem(source_dir.path().to_path_buf()),
            ScanContext::new(ScanGeneration(2), CancellationToken::new()),
            captureport_ingest::MediaRules::default(),
            tx,
            catalog,
        );
        // A quick sample is only a possible match; retain the prior session
        // for review without claiming the source bytes were fully verified.
        let prior_import = rx.into_iter().find_map(|message| match message {
            ScanMessage::Event(event) => match *event {
                AppEvent::ImportStatusChanged {
                    status: captureport_core::ImportStatus::PossibleDuplicate,
                    prior_import,
                    ..
                } => Some(prior_import),
                _ => None,
            },
            _ => None,
        });
        let prior = prior_import
            .expect("the rescan should identify the possible duplicate")
            .expect("a possible duplicate should name the prior session");
        assert_eq!(prior.session_id, recorded_session);
        assert!(!prior.imported_at.is_empty());
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

    /// The P0 this critique found: media the engine had *not* cleared as safe
    /// was labelled "Imported". The label must differ, and must say what matched.
    #[test]
    fn status_line_names_the_prior_import_and_never_reads_as_imported() {
        let mut item =
            captureport_core::MediaItem::new(MediaId(1), SourceId(1), "DCIM/IMG0001.JPG", 1024);
        item.import_status = captureport_core::ImportStatus::PossibleDuplicate;
        assert_eq!(import_status_line(&item), "! Possible duplicate");
        assert_ne!(
            import_status_line(&item),
            import_status_label(&captureport_core::ImportStatus::Imported)
        );
        item.prior_import = Some(captureport_core::PriorImport {
            session_id: 12,
            imported_at: "2026-09-27T20:35:29Z".into(),
            destination: "Pictures/IMG0001.JPG".into(),
        });
        assert_eq!(
            import_status_line(&item),
            "! Possible duplicate · session 12 · 2026-09-27"
        );
    }

    /// Deleting originals is the one irreversible action in the product, so the
    /// scope shown to the user and the scope that runs must be the same
    /// predicate — this pins that predicate's edges.
    #[test]
    fn deletion_scope_covers_only_sources_whose_required_copies_landed() {
        use captureport_core::{SourceId, SourceType};
        use captureport_ingest::{CopyResult, ImportItemState, ImportResult, ItemResult};

        let copy = |media: u64, index: u64, required: bool| PlannedCopy {
            destination_root: PathBuf::from("/library"),
            final_destination: PathBuf::from(format!("/library/{media}-{index}.jpg")),
            temporary_destination: PathBuf::from(format!("/library/.partial/{media}-{index}.jpg")),
            expected_size: 1024,
            required,
            status: PlanStatus::Ready,
        };
        let planned = |media: u64, copies: Vec<PlannedCopy>| PlannedImport {
            media_id: MediaId(media),
            source: MediaLocator(format!("DCIM/{media}.jpg")),
            source_name: format!("{media}.jpg"),
            expected_size: 1024,
            effective_time: DateTime::parse_from_rfc3339("2026-09-27T12:00:00Z")
                .expect("valid timestamp"),
            sequence: media,
            session: 1,
            copies,
            status: PlanStatus::Ready,
        };
        let observed =
            |media: u64, required: ImportItemState, optional: ImportItemState| ItemResult {
                media_id: MediaId(media),
                copies: vec![
                    CopyResult {
                        destination: PathBuf::from(format!("/library/{media}-0.jpg")),
                        state: required,
                        error: None,
                    },
                    CopyResult {
                        destination: PathBuf::from(format!("/library/{media}-1.jpg")),
                        state: optional,
                        error: None,
                    },
                ],
            };

        let plan = ImportPlan {
            source: captureport_core::SourceIdentity {
                id: SourceId(1),
                source_type: SourceType::Filesystem,
                stable_id: None,
                serial: None,
                manufacturer: None,
                model: None,
                volume_uuid: None,
                display_name: Some("Card".into()),
            },
            preset_name: "Everyday".into(),
            verification: VerificationMode::Standard,
            items: vec![
                planned(1, vec![copy(1, 0, true)]),
                planned(2, vec![copy(2, 0, true), copy(2, 1, false)]),
                planned(3, vec![copy(3, 0, true)]),
            ],
        };
        let result = ImportResult {
            cancelled: false,
            items: vec![
                observed(1, ImportItemState::Completed, ImportItemState::Completed),
                // A failed optional backup copy must not block deleting the source.
                observed(2, ImportItemState::Completed, ImportItemState::Failed),
                // A failed required copy must block it.
                observed(3, ImportItemState::Failed, ImportItemState::Completed),
            ],
        };

        let deletable = deletable_sources(&plan, &result, false);
        let ids: Vec<u64> = deletable.iter().map(|item| item.media_id.0).collect();
        assert_eq!(ids, vec![1, 2]);
        assert_eq!(
            deletable.iter().map(|item| item.expected_size).sum::<u64>(),
            2048
        );
    }
}

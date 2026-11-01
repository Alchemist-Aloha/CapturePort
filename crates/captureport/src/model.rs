use crate::*;

pub(crate) struct HistoryEntry {
    pub(crate) detail: SessionDetail,
    pub(crate) source_name: String,
}
#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum Page {
    Browser,
    Preview,
    History,
    Recovery,
    Settings,
}
pub(crate) struct SettingsInputs {
    pub(crate) preset_name: Entity<text_input::TextInput>,
    pub(crate) photo_root: Entity<text_input::TextInput>,
    pub(crate) photo_folder: Entity<text_input::TextInput>,
    pub(crate) video_root: Entity<text_input::TextInput>,
    pub(crate) video_folder: Entity<text_input::TextInput>,
    pub(crate) filename: Entity<text_input::TextInput>,
    pub(crate) clock_seconds: Entity<text_input::TextInput>,
    pub(crate) timezone_seconds: Entity<text_input::TextInput>,
    pub(crate) gap_minutes: Entity<text_input::TextInput>,
    pub(crate) backup_photo_root: Entity<text_input::TextInput>,
    pub(crate) backup_video_root: Entity<text_input::TextInput>,
    pub(crate) source_alias: Entity<text_input::TextInput>,
    pub(crate) include_photo: Entity<text_input::TextInput>,
    pub(crate) include_video: Entity<text_input::TextInput>,
    pub(crate) exclude_extensions: Entity<text_input::TextInput>,
    pub(crate) exclude_folders: Entity<text_input::TextInput>,
    pub(crate) ignore_types: Entity<text_input::TextInput>,
}
pub(crate) struct Startup {
    pub(crate) catalog: CatalogHandle,
    pub(crate) cache_dir: PathBuf,
    pub(crate) config_path: PathBuf,
    pub(crate) ui_path: PathBuf,
    pub(crate) gallery_names_path: PathBuf,
    pub(crate) ui: UiPreferences,
    pub(crate) preset: ImportPreset,
    pub(crate) initial_source: Option<SourceRequest>,
    pub(crate) demo_importing: bool,
}
pub(crate) fn import_phase_label(phase: captureport_core::ImportPhase) -> &'static str {
    use captureport_core::ImportPhase;
    match phase {
        ImportPhase::Downloading => "Downloading from camera",
        ImportPhase::ReadingSource => "Reading source",
        ImportPhase::Copying => "Copying",
        ImportPhase::Verifying => "Verifying",
        ImportPhase::Committing => "Publishing verified copy",
        ImportPhase::Recording => "Saving import history",
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PreviewReadiness {
    Loading,
    Ready,
    Unavailable,
}
pub(crate) fn preview_readiness(
    metadata: &captureport_core::MetadataState,
    preview_metadata: &captureport_core::MetadataState,
    has_thumbnail: bool,
    thumbnail_failed: bool,
) -> PreviewReadiness {
    use captureport_core::MetadataState;
    match (metadata, preview_metadata) {
        (MetadataState::Failed(_), _) | (_, MetadataState::Failed(_)) => {
            PreviewReadiness::Unavailable
        }
        (MetadataState::Pending, _) | (_, MetadataState::Pending) => PreviewReadiness::Loading,
        _ if has_thumbnail => PreviewReadiness::Ready,
        _ if thumbnail_failed => PreviewReadiness::Unavailable,
        _ => PreviewReadiness::Loading,
    }
}
#[derive(Default)]
pub(crate) struct ThumbnailProgress {
    pub(crate) ready: usize,
    pub(crate) unavailable: usize,
    pub(crate) total: usize,
}
impl ThumbnailProgress {
    pub(crate) fn resolved(&self) -> usize {
        self.ready + self.unavailable
    }
}

#[derive(Clone)]
pub(crate) struct GalleryGroup {
    pub(crate) key: String,
    pub(crate) title: String,
    pub(crate) session: u32,
    pub(crate) ids: Vec<MediaId>,
    pub(crate) all_ids: Vec<MediaId>,
}
pub(crate) struct GalleryRow {
    pub(crate) header: Option<(String, String)>,
    pub(crate) ids: Vec<MediaId>,
}
pub(crate) fn visible_capture_ids(
    visible_members: impl IntoIterator<Item = MediaId>,
    bundle_owner: &HashMap<MediaId, MediaId>,
) -> Vec<MediaId> {
    let mut seen = HashSet::new();
    visible_members
        .into_iter()
        .map(|id| bundle_owner.get(&id).copied().unwrap_or(id))
        .filter(|id| seen.insert(*id))
        .collect()
}
/// Streams same-folder, same-stem JPEG siblings while a scan is still running.
/// A RAW with a JPEG pair never needs its own preview decoded, so this index
/// lets thumbnail requests skip it before the full bundle pass runs.
/// ponytail: one slot per stem, so a stem with several RAWs may mis-skip during
/// the stream; `rebuild_bundles` replaces it with the authoritative pairs at the
/// end of the scan.
#[derive(Default)]
pub(crate) struct PairIndex {
    pub(crate) raw_by_stem: HashMap<(String, String), MediaId>,
    pub(crate) jpeg_by_stem: HashMap<(String, String), MediaId>,
    pub(crate) paired: HashMap<MediaId, MediaId>,
}
impl PairIndex {
    pub(crate) fn clear(&mut self) {
        self.raw_by_stem.clear();
        self.jpeg_by_stem.clear();
        self.paired.clear();
    }

    pub(crate) fn note(&mut self, id: MediaId, media_type: MediaType, path: &str) {
        let key = captureport_ingest::bundle_key(path);
        match media_type {
            MediaType::Raw => {
                if let Some(jpeg) = self.jpeg_by_stem.get(&key).copied() {
                    self.paired.insert(id, jpeg);
                }
                self.raw_by_stem.insert(key, id);
            }
            MediaType::Jpeg => {
                if let Some(raw) = self.raw_by_stem.get(&key).copied() {
                    self.paired.insert(raw, id);
                }
                self.jpeg_by_stem.insert(key, id);
            }
            _ => {}
        }
    }

    /// Replace the streaming guess with the authoritative bundle result.
    pub(crate) fn replace(&mut self, pairs: impl IntoIterator<Item = (MediaId, MediaId)>) {
        self.paired = pairs.into_iter().collect();
    }

    pub(crate) fn has_pair(&self, id: MediaId) -> bool {
        self.paired.contains_key(&id)
    }
}
pub(crate) fn blocks_import(status: PlanStatus) -> bool {
    !status.can_execute() && status != PlanStatus::Skipped
}
/// Whether a bundle is one display item. In separate mode a RAW+JPEG pair is
/// listed as two files, so only non-RAW bundles stay merged.
pub(crate) fn shows_as_bundle(
    bundle: &captureport_ingest::MediaBundle,
    merge_raw_jpeg: bool,
) -> bool {
    bundle.members.len() > 1 && (merge_raw_jpeg || bundle.bundle_type != BundleType::RawJpeg)
}
pub(crate) fn append_ready_ids(displayed: &mut Vec<MediaId>, ready: Vec<MediaId>) {
    let previous = displayed.iter().copied().collect::<HashSet<_>>();
    let ready_set = ready.iter().copied().collect::<HashSet<_>>();
    displayed.retain(|id| ready_set.contains(id));
    displayed.extend(ready.into_iter().filter(|id| !previous.contains(id)));
}
/// Keep session boundaries chronological, but order their tiles as displayed.
pub(crate) fn order_gallery_members(groups: &mut [GalleryGroup], displayed: &[MediaId]) {
    let ranks = displayed
        .iter()
        .enumerate()
        .map(|(rank, id)| (*id, rank))
        .collect::<HashMap<_, _>>();
    for group in groups {
        group
            .ids
            .sort_by_key(|id| ranks.get(id).copied().unwrap_or(usize::MAX));
    }
}

/// Common UTC offsets offered by the timezone menu.
pub(crate) const TIMEZONE_CHOICES: &[(&str, i32)] = &[
    ("UTC-12:00", -43_200),
    ("UTC-11:00", -39_600),
    ("UTC-10:00", -36_000),
    ("UTC-09:30", -34_200),
    ("UTC-09:00", -32_400),
    ("UTC-08:00", -28_800),
    ("UTC-07:00", -25_200),
    ("UTC-06:00", -21_600),
    ("UTC-05:00", -18_000),
    ("UTC-04:00", -14_400),
    ("UTC-03:30", -12_600),
    ("UTC-03:00", -10_800),
    ("UTC-02:00", -7_200),
    ("UTC-01:00", -3_600),
    ("UTC+00:00", 0),
    ("UTC+01:00", 3_600),
    ("UTC+02:00", 7_200),
    ("UTC+03:00", 10_800),
    ("UTC+03:30", 12_600),
    ("UTC+04:00", 14_400),
    ("UTC+04:30", 16_200),
    ("UTC+05:00", 18_000),
    ("UTC+05:30", 19_800),
    ("UTC+05:45", 20_700),
    ("UTC+06:00", 21_600),
    ("UTC+06:30", 23_400),
    ("UTC+07:00", 25_200),
    ("UTC+08:00", 28_800),
    ("UTC+08:45", 31_500),
    ("UTC+09:00", 32_400),
    ("UTC+09:30", 34_200),
    ("UTC+10:00", 36_000),
    ("UTC+10:30", 37_800),
    ("UTC+11:00", 39_600),
    ("UTC+12:00", 43_200),
    ("UTC+12:45", 45_900),
    ("UTC+13:00", 46_800),
    ("UTC+14:00", 50_400),
];
pub(crate) fn gallery_source_key(identity: captureport_core::SourceIdentity) -> String {
    identity
        .stable_id
        .or(identity.volume_uuid)
        .or(identity.display_name)
        .unwrap_or_else(|| format!("{:?}", identity.source_type))
}

pub(crate) fn gallery_name_key(source: &str, path: &str) -> String {
    format!("v2:{}", serde_json::json!([source, path]))
}

/// Empty v2 names explicitly reset a rename, masking retained legacy entries.
pub(crate) fn saved_gallery_name<'a>(
    names: &'a HashMap<String, String>,
    source: &str,
    paths: &[&str],
    session: u32,
) -> Option<&'a str> {
    let saved = paths
        .iter()
        .filter_map(|path| names.get(&gallery_name_key(source, path)))
        .collect::<Vec<_>>();
    if !saved.is_empty() {
        return saved
            .into_iter()
            .find(|name| !name.is_empty())
            .map(String::as_str);
    }
    for path in paths {
        if let Some(name) = names.get(&format!("{source}|{path}|{session}")) {
            return Some(name);
        }
    }
    // Recover ordinal drift only when every matching legacy entry agrees.
    let mut matches = names
        .iter()
        .filter(|(key, _)| {
            key.rsplit_once('|').is_some_and(|(prefix, number)| {
                number.parse::<u32>().is_ok()
                    && paths
                        .iter()
                        .any(|path| prefix == format!("{source}|{path}"))
            })
        })
        .map(|(_, name)| name.as_str());
    let name = matches.next()?;
    matches.all(|other| other == name).then_some(name)
}
/// Append a, b, c, ... to default gallery names that share a date.
pub(crate) fn suffix_duplicate_titles(
    groups: &mut [GalleryGroup],
    is_custom: impl Fn(&str) -> bool,
) {
    let mut totals = HashMap::<String, usize>::new();
    for group in groups.iter() {
        if !is_custom(&group.key) {
            *totals.entry(group.title.clone()).or_default() += 1;
        }
    }
    let mut seen = HashMap::<String, usize>::new();
    for group in groups.iter_mut() {
        if is_custom(&group.key) || totals.get(&group.title).copied().unwrap_or(1) < 2 {
            continue;
        }
        let index = seen.entry(group.title.clone()).or_default();
        let mut number = *index;
        let mut suffix = Vec::new();
        loop {
            suffix.push((b'a' + (number % 26) as u8) as char);
            if number < 26 {
                break;
            }
            number = number / 26 - 1;
        }
        let suffix = suffix.into_iter().rev().collect::<String>();
        group.title.push_str(&suffix);
        *index += 1;
    }
}
pub(crate) fn timezone_label(offset: Option<i32>) -> String {
    match offset {
        None => i18n::tr("Capture metadata"),
        Some(seconds) => {
            let sign = if seconds < 0 { '-' } else { '+' };
            let absolute = seconds.abs();
            format!(
                "UTC{sign}{:02}:{:02}",
                absolute / 3600,
                (absolute % 3600) / 60
            )
        }
    }
}
/// Format a source file's modification time in the selected timezone.
pub(crate) fn format_file_time(seconds: u64, offset: Option<i32>) -> String {
    let Some(utc) = Utc.timestamp_opt(seconds as i64, 0).single() else {
        return String::new();
    };
    let time = match offset.and_then(FixedOffset::east_opt) {
        Some(offset) => utc.with_timezone(&offset),
        None => utc.with_timezone(&Local).fixed_offset(),
    };
    time.format("%Y-%m-%d %H:%M").to_string()
}
/// PageUp/PageDown target offset, clamped to the scrollable range.
pub(crate) fn page_scroll_target(current: f32, step: f32, limit: f32) -> f32 {
    (current + step).clamp(-limit.max(0.), 0.)
}
/// Format a stored RFC 3339 session timestamp for display, matching the media
/// cards' local `YYYY-MM-DD HH:MM` shape. Unparseable values are shown as stored
/// rather than hidden.
pub(crate) fn format_session_time(started_at: &str) -> String {
    DateTime::parse_from_rfc3339(started_at)
        .map(|time| {
            time.with_timezone(&Local)
                .format("%Y-%m-%d %H:%M")
                .to_string()
        })
        .unwrap_or_else(|_| started_at.to_owned())
}
/// Compact modification time for media cards.
pub(crate) fn format_file_time_compact(seconds: u64, offset: Option<i32>) -> String {
    let Some(utc) = Utc.timestamp_opt(seconds as i64, 0).single() else {
        return String::new();
    };
    let time = match offset.and_then(FixedOffset::east_opt) {
        Some(offset) => utc.with_timezone(&offset),
        None => utc.with_timezone(&Local).fixed_offset(),
    };
    time.format("%y-%m-%d %H:%M").to_string()
}
#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(default)]
pub(crate) struct UiPreferences {
    /// None follows LC_ALL / LC_MESSAGES / LANG.
    pub(crate) language: Option<i18n::Language>,
    pub(crate) dark_mode: bool,
    pub(crate) scheme: ColorScheme,
    pub(crate) thumbnail_size: u8,
    /// Show a RAW+JPEG pair as one item. Off shows each file separately.
    pub(crate) merge_raw_jpeg: bool,
}
impl Default for UiPreferences {
    fn default() -> Self {
        Self {
            language: None,
            dark_mode: false,
            scheme: ColorScheme::default(),
            thumbnail_size: 2,
            merge_raw_jpeg: true,
        }
    }
}
impl SettingsInputs {
    pub(crate) fn new(preset: &ImportPreset, cx: &mut Context<Browser>) -> Self {
        let field = |value: String, cx: &mut Context<Browser>| {
            cx.new(|cx| text_input::TextInput::new(value, "", cx))
        };
        Self {
            preset_name: field(String::new(), cx),
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
            include_photo: field(preset.media_rules.include_photo.join(", "), cx),
            include_video: field(preset.media_rules.include_video.join(", "), cx),
            exclude_extensions: field(preset.media_rules.exclude.join(", "), cx),
            exclude_folders: field(preset.media_rules.exclude_folders.join(", "), cx),
            ignore_types: field(preset.media_rules.ignore.join(", "), cx),
        }
    }
    pub(crate) fn sync(&self, preset: &ImportPreset, cx: &mut Context<Browser>) {
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
        self.include_photo.update(cx, |input, cx| {
            input.set_value(preset.media_rules.include_photo.join(", "), cx)
        });
        self.include_video.update(cx, |input, cx| {
            input.set_value(preset.media_rules.include_video.join(", "), cx)
        });
        self.exclude_extensions.update(cx, |input, cx| {
            input.set_value(preset.media_rules.exclude.join(", "), cx)
        });
        self.exclude_folders.update(cx, |input, cx| {
            input.set_value(preset.media_rules.exclude_folders.join(", "), cx)
        });
        self.ignore_types.update(cx, |input, cx| {
            input.set_value(preset.media_rules.ignore.join(", "), cx)
        });
    }
    pub(crate) fn read(
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
                Err(i18n::tr("Destination roots must be absolute paths"))
            }
        };
        let photo_folder = self.photo_folder.read(cx).value();
        let video_folder = self.video_folder.read(cx).value();
        let filename = self.filename.read(cx).value();
        for (label, template, is_filename, video) in [
            ("Photo folder", &photo_folder, false, false),
            ("Video folder", &video_folder, false, true),
            ("Filename", &filename, true, false),
        ] {
            template_help::example(template, is_filename, video)
                .map_err(|error| format!("{}: {error}", i18n::tr(label)))?;
        }
        let clock_seconds = self
            .clock_seconds
            .read(cx)
            .value()
            .parse::<i64>()
            .map_err(|_| i18n::tr("Clock correction must be a number of seconds"))?;
        let timezone = self.timezone_seconds.read(cx).value();
        let assumed_utc_offset_seconds = if timezone.trim().is_empty() {
            None
        } else {
            let value = timezone
                .parse::<i32>()
                .map_err(|_| i18n::tr("Timezone offset must be seconds east of UTC"))?;
            if !(-86_400..=86_400).contains(&value) {
                return Err(i18n::tr("Timezone offset is out of range"));
            }
            Some(value)
        };
        let backup_photo = self.backup_photo_root.read(cx).value();
        let backup_video = self.backup_video_root.read(cx).value();
        let backup = if backup_photo.trim().is_empty() && backup_video.trim().is_empty() {
            None
        } else {
            if backup_photo.trim().is_empty() || backup_video.trim().is_empty() {
                return Err(i18n::tr("Set both backup roots or clear both"));
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
                .map_err(|_| i18n::tr("Session gap must be a number of minutes"))?;
            if threshold_minutes == 0 {
                return Err(i18n::tr("Session gap must be greater than zero"));
            }
            preset.grouping = Grouping::TimeGap { threshold_minutes };
        }
        preset.backup = backup;
        let split = |value: String| -> Vec<String> {
            value
                .split(',')
                .map(|entry| entry.trim().to_string())
                .filter(|entry| !entry.is_empty())
                .collect()
        };
        preset.media_rules = captureport_ingest::MediaRules {
            include_photo: split(self.include_photo.read(cx).value()),
            include_video: split(self.include_video.read(cx).value()),
            exclude: split(self.exclude_extensions.read(cx).value()),
            exclude_folders: split(self.exclude_folders.read(cx).value()),
            ignore: split(self.ignore_types.read(cx).value()),
        };
        Ok(preset)
    }
}
pub(crate) fn capture_order_key(
    item: &captureport_core::MediaItem,
    preset: &ImportPreset,
) -> (DateTime<FixedOffset>, String, String, MediaId) {
    (
        effective_time(capture_time(item), preset),
        item.source_name.clone(),
        item.source_path.clone(),
        item.id,
    )
}

pub(crate) fn capture_time(item: &captureport_core::MediaItem) -> DateTime<FixedOffset> {
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
pub(crate) fn format_size(bytes: u64) -> String {
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

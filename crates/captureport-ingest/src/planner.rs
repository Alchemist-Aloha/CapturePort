//! Deterministic, side effect free import planning.

use crate::{CollisionPolicy, Grouping, ImportPreset, Template, TemplateContext, VerificationMode};
use captureport_core::{
    MediaId, MediaItem, MediaLocator, MediaSource, MediaType, MetadataState, SourceIdentity,
};
use chrono::{DateTime, Datelike, Duration, FixedOffset};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct PlanInput {
    pub item: MediaItem,
    pub capture_time: DateTime<FixedOffset>,
    /// Gallery display name for the item's shooting session, when one exists.
    /// `None` renders `{session_name}` as `unknown`.
    pub session_name: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlanStatus {
    Ready,
    ExistingIdentical,
    Skipped,
    DestinationCollision,
    InvalidPath,
    DestinationUnavailable,
    InsufficientDiskSpace,
    UnsupportedSource,
}

impl PlanStatus {
    pub fn can_execute(self) -> bool {
        matches!(self, Self::Ready | Self::ExistingIdentical)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlannerError {
    InvalidInput(String),
}

impl std::fmt::Display for PlannerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput(detail) => write!(f, "Invalid import planning input: {detail}"),
        }
    }
}
impl std::error::Error for PlannerError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlannedCopy {
    pub destination_root: PathBuf,
    pub final_destination: PathBuf,
    pub temporary_destination: PathBuf,
    pub expected_size: u64,
    pub required: bool,
    pub status: PlanStatus,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlannedImport {
    pub media_id: MediaId,
    pub source: MediaLocator,
    pub source_name: String,
    pub expected_size: u64,
    pub effective_time: DateTime<FixedOffset>,
    pub sequence: u64,
    pub session: u32,
    pub copies: Vec<PlannedCopy>,
    pub status: PlanStatus,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportPlan {
    pub source: SourceIdentity,
    pub preset_name: String,
    pub verification: VerificationMode,
    pub items: Vec<PlannedImport>,
}

pub struct ImportPlanner;

impl ImportPlanner {
    pub fn build(
        source: &dyn MediaSource,
        selected: Vec<PlanInput>,
        preset: &ImportPreset,
    ) -> ImportPlan {
        Self::build_with_explicit_members(source, selected, preset, &HashSet::new())
    }

    pub fn build_with_explicit_members(
        source: &dyn MediaSource,
        selected: Vec<PlanInput>,
        preset: &ImportPreset,
        explicit_members: &HashSet<MediaId>,
    ) -> ImportPlan {
        let identity = source.identity();
        let selected = selected
            .into_iter()
            .filter(|input| !preset.media_rules.excludes_folder(&input.item.source_path))
            .collect();
        let mut selected = crate::bundle::apply_bundle_policy_with_overrides(
            selected,
            preset.bundle_policy,
            explicit_members,
        );
        selected.sort_by(|a, b| {
            effective_time(a.capture_time, preset)
                .cmp(&effective_time(b.capture_time, preset))
                .then_with(|| a.item.source_name.cmp(&b.item.source_name))
                .then_with(|| a.item.source_path.cmp(&b.item.source_path))
                .then_with(|| a.item.id.cmp(&b.item.id))
        });

        let effective: Vec<_> = selected
            .iter()
            .map(|input| (input, effective_time(input.capture_time, preset)))
            .collect();
        let sessions = session_numbers(
            &effective.iter().map(|(_, time)| *time).collect::<Vec<_>>(),
            &preset.grouping,
        );
        let mut counts = HashMap::<u32, u64>::new();
        let mut planned = Vec::with_capacity(selected.len());
        let mut generated = HashMap::<PathBuf, usize>::new();
        let mut roots = BTreeMap::<PathBuf, (u64, bool)>::new();

        // Treat a RAW+JPEG pair as one unit: the RAW is planned first, the JPEG
        // shares its sequence, and the JPEG is copied beside the RAW with a
        // `.jpg` extension even when the filename template renames files.
        let sidecar_of = raw_jpeg_sidecars(&selected);
        let jpeg_of_raw: HashMap<MediaId, MediaId> =
            sidecar_of.iter().map(|(jpeg, raw)| (*raw, *jpeg)).collect();
        let index_of: HashMap<MediaId, usize> = effective
            .iter()
            .enumerate()
            .map(|(index, (input, _))| (input.item.id, index))
            .collect();
        let mut handled = HashSet::new();
        let mut units: Vec<Vec<usize>> = Vec::new();
        for (index, (input, _)) in effective.iter().enumerate() {
            if sidecar_of.contains_key(&input.item.id) || !handled.insert(input.item.id) {
                continue;
            }
            let mut unit = vec![index];
            if let Some(jpeg_id) = jpeg_of_raw.get(&input.item.id)
                && let Some(&jpeg_index) = index_of.get(jpeg_id)
            {
                handled.insert(*jpeg_id);
                unit.push(jpeg_index);
            }
            units.push(unit);
        }

        for unit in units {
            let (primary_input, primary_time) = effective[unit[0]];
            let session = sessions[unit[0]];
            let sequence = {
                let count = counts.entry(session).or_default();
                *count += 1;
                *count
            };
            let primary_copies = make_copies(
                source,
                primary_input,
                primary_time,
                sequence,
                session,
                preset,
                &identity,
                &mut generated,
                &mut roots,
                None,
            );
            let sidecar = (unit.len() > 1).then(|| {
                (
                    primary_copies[0].final_destination.clone(),
                    primary_copies
                        .get(1)
                        .map(|copy| copy.final_destination.clone()),
                )
            });
            for (position, &member) in unit.iter().enumerate() {
                let (input, time) = effective[member];
                let copies = if position == 0 {
                    primary_copies.clone()
                } else {
                    make_copies(
                        source,
                        input,
                        time,
                        sequence,
                        session,
                        preset,
                        &identity,
                        &mut generated,
                        &mut roots,
                        sidecar
                            .as_ref()
                            .map(|(primary, backup)| (primary.as_path(), backup.as_deref())),
                    )
                };
                let status = copies
                    .iter()
                    .filter(|copy| copy.required)
                    .map(|copy| copy.status)
                    .find(|status| *status != PlanStatus::Ready)
                    .unwrap_or(PlanStatus::Ready);
                planned.push(PlannedImport {
                    media_id: input.item.id,
                    source: input.item.locator.clone(),
                    source_name: input.item.source_name.clone(),
                    expected_size: input.item.size,
                    effective_time: time,
                    sequence,
                    session,
                    copies,
                    status,
                });
            }
        }

        // Free space is checked after all paths and sizes are known. A root is
        // marked unavailable before this pass, so statvfs cannot turn a bad
        // destination into a misleading space error.
        for (root, (bytes, available)) in roots {
            if !available {
                continue;
            }
            let free = free_space(&root).unwrap_or(0);
            if free < bytes {
                for item in &mut planned {
                    for copy in &mut item.copies {
                        if copy.final_destination.starts_with(&root)
                            && copy.status == PlanStatus::Ready
                        {
                            copy.status = PlanStatus::InsufficientDiskSpace;
                            if copy.required {
                                item.status = PlanStatus::InsufficientDiskSpace;
                            }
                        }
                    }
                }
            }
        }
        ImportPlan {
            source: identity,
            preset_name: preset.name.clone(),
            verification: preset.verification,
            items: planned,
        }
    }
}

pub fn effective_time(time: DateTime<FixedOffset>, preset: &ImportPreset) -> DateTime<FixedOffset> {
    let interpreted = preset
        .time_correction
        .assumed_utc_offset_seconds
        .and_then(FixedOffset::east_opt)
        .and_then(|offset| time.naive_local().and_local_timezone(offset).single())
        .unwrap_or(time);
    interpreted
        .checked_add_signed(Duration::seconds(preset.time_correction.offset_seconds))
        .unwrap_or(interpreted)
}

pub fn session_numbers(times: &[DateTime<FixedOffset>], grouping: &Grouping) -> Vec<u32> {
    if times.is_empty() {
        return Vec::new();
    }
    let mut result = Vec::with_capacity(times.len());
    let mut session = 1u32;
    let mut previous = times[0];
    for time in times {
        if !result.is_empty() {
            let split = match grouping {
                Grouping::None => false,
                Grouping::Day => time.date_naive() != previous.date_naive(),
                Grouping::Week => time.iso_week() != previous.iso_week(),
                Grouping::Month => {
                    (time.year(), time.month()) != (previous.year(), previous.month())
                }
                Grouping::Year => time.year() != previous.year(),
                Grouping::TimeGap { threshold_minutes } => {
                    time.signed_duration_since(previous)
                        > Duration::minutes(*threshold_minutes as i64)
                }
            };
            if split {
                session = session.saturating_add(1);
            }
        }
        result.push(session);
        previous = *time;
    }
    result
}

/// Map each selected JPEG to the RAW sharing its directory and stem.
fn raw_jpeg_sidecars(inputs: &[PlanInput]) -> HashMap<MediaId, MediaId> {
    let items: Vec<MediaItem> = inputs.iter().map(|input| input.item.clone()).collect();
    let types: HashMap<MediaId, MediaType> = items
        .iter()
        .map(|item| (item.id, item.media_type))
        .collect();
    let mut sidecars = HashMap::new();
    for bundle in crate::group_media(&items).bundles {
        if bundle.bundle_type != crate::BundleType::RawJpeg {
            continue;
        }
        let raw = bundle
            .members
            .iter()
            .copied()
            .find(|id| types.get(id) == Some(&MediaType::Raw));
        let jpeg = bundle
            .members
            .iter()
            .copied()
            .find(|id| types.get(id) == Some(&MediaType::Jpeg));
        if let (Some(raw), Some(jpeg)) = (raw, jpeg) {
            sidecars.insert(jpeg, raw);
        }
    }
    sidecars
}

#[allow(clippy::too_many_arguments)]
fn make_copies(
    source: &dyn MediaSource,
    input: &PlanInput,
    time: DateTime<FixedOffset>,
    sequence: u64,
    session: u32,
    preset: &ImportPreset,
    identity: &SourceIdentity,
    generated: &mut HashMap<PathBuf, usize>,
    roots: &mut BTreeMap<PathBuf, (u64, bool)>,
    sidecar: Option<(&Path, Option<&Path>)>,
) -> Vec<PlannedCopy> {
    let rule = if input.item.media_type == MediaType::Video {
        &preset.video
    } else {
        &preset.photo
    };
    if input.item.source_id != identity.id || input.item.media_type == MediaType::Unknown {
        return vec![invalid_copy(
            &rule.root,
            rule.root.join(&input.item.source_name),
            input.item.size,
            true,
            PlanStatus::UnsupportedSource,
        )];
    }
    if let Some((primary_path, backup_path)) = sidecar {
        let mut copies = vec![plan_copy(
            source,
            input,
            primary_path.with_extension("jpg"),
            &rule.root,
            true,
            preset.collision,
            generated,
            roots,
        )];
        if let Some(backup) = &preset.backup
            && let Some(backup_path) = backup_path
        {
            let backup_rule = if input.item.media_type == MediaType::Video {
                &backup.video
            } else {
                &backup.photo
            };
            copies.push(plan_copy(
                source,
                input,
                backup_path.with_extension("jpg"),
                &backup_rule.root,
                backup.required,
                preset.collision,
                generated,
                roots,
            ));
        }
        return copies;
    }
    let metadata = match &input.item.metadata {
        MetadataState::Ready(metadata) => Some(metadata),
        _ => None,
    };
    let camera_model = metadata
        .and_then(|m| m.camera_model.as_deref())
        .filter(|v| !v.is_empty())
        .or(identity.model.as_deref());
    let camera = camera_model
        .or(identity.display_name.as_deref())
        .unwrap_or("");
    let (stem, extension) = split_name(&input.item.source_name);
    let context = TemplateContext {
        timestamp: time,
        camera,
        camera_make: metadata
            .and_then(|m| m.camera_make.as_deref())
            .filter(|v| !v.is_empty())
            .or(identity.manufacturer.as_deref())
            .unwrap_or(""),
        camera_model: camera_model.unwrap_or(""),
        camera_serial: metadata
            .and_then(|m| m.camera_serial.as_deref())
            .filter(|v| !v.is_empty())
            .or(identity.serial.as_deref())
            .unwrap_or(""),
        original_name: &input.item.source_name,
        original_stem: stem,
        extension,
        media_type: if input.item.media_type == MediaType::Video {
            "video"
        } else {
            "photo"
        },
        sequence,
        session,
        session_name: input.session_name.as_deref().unwrap_or(""),
        metadata,
        file_size: input.item.size,
        source_name: identity.display_name.as_deref().unwrap_or(""),
    };
    let folder = match Template::parse(&rule.folder_template)
        .and_then(|t| t.render_relative_path(&context))
    {
        Ok(folder) => folder,
        Err(_) => {
            return vec![invalid_copy(
                &rule.root,
                rule.root.clone(),
                input.item.size,
                true,
                PlanStatus::InvalidPath,
            )];
        }
    };
    let filename = match Template::parse(&preset.filename_template)
        .and_then(|t| t.render_filename(&context))
    {
        Ok(name) => name,
        Err(_) => {
            return vec![invalid_copy(
                &rule.root,
                rule.root.clone(),
                input.item.size,
                true,
                PlanStatus::InvalidPath,
            )];
        }
    };
    let primary = rule
        .root
        .join(grouped_folder(&folder, &preset.grouping, time))
        .join(filename);
    let mut copies = vec![plan_copy(
        source,
        input,
        primary,
        &rule.root,
        true,
        preset.collision,
        generated,
        roots,
    )];
    if let Some(backup) = &preset.backup {
        let backup_rule = if input.item.media_type == MediaType::Video {
            &backup.video
        } else {
            &backup.photo
        };
        let backup_folder = Template::parse(&backup_rule.folder_template)
            .and_then(|t| t.render_relative_path(&context));
        let backup_name =
            Template::parse(&preset.filename_template).and_then(|t| t.render_filename(&context));
        let copy = match (backup_folder, backup_name) {
            (Ok(folder), Ok(name)) => {
                let folder = grouped_folder(&folder, &preset.grouping, time);
                plan_copy(
                    source,
                    input,
                    backup_rule.root.join(folder).join(name),
                    &backup_rule.root,
                    backup.required,
                    preset.collision,
                    generated,
                    roots,
                )
            }
            _ => invalid_copy(
                &backup_rule.root,
                backup_rule.root.clone(),
                input.item.size,
                backup.required,
                PlanStatus::InvalidPath,
            ),
        };
        copies.push(copy);
    }
    copies
}

fn grouped_folder(folder: &str, grouping: &Grouping, time: DateTime<FixedOffset>) -> PathBuf {
    let mut path = PathBuf::from(folder);
    match grouping {
        // Time-gap sessions affect numbering; their folders are opt-in through
        // {session} in the template, so the browser's gap slider adds no layer.
        Grouping::None | Grouping::TimeGap { .. } => {}
        Grouping::Day => path.push(time.format("%Y%m%d").to_string()),
        Grouping::Week => path.push(format!(
            "{}-W{:02}",
            time.iso_week().year(),
            time.iso_week().week()
        )),
        Grouping::Month => path.push(time.format("%Y%m").to_string()),
        Grouping::Year => path.push(time.format("%Y").to_string()),
    }
    path
}

fn invalid_copy(
    root: &Path,
    path: PathBuf,
    size: u64,
    required: bool,
    status: PlanStatus,
) -> PlannedCopy {
    let temporary_destination = path.with_file_name(format!(
        ".captureport-{}.{}.partial",
        path.file_name().unwrap_or_default().to_string_lossy(),
        Uuid::new_v4()
    ));
    PlannedCopy {
        destination_root: absolute_root(root),
        temporary_destination,
        final_destination: path,
        expected_size: size,
        required,
        status,
    }
}

#[allow(clippy::too_many_arguments)]
fn plan_copy(
    source: &dyn MediaSource,
    input: &PlanInput,
    mut path: PathBuf,
    destination_root: &Path,
    required: bool,
    policy: CollisionPolicy,
    generated: &mut HashMap<PathBuf, usize>,
    roots: &mut BTreeMap<PathBuf, (u64, bool)>,
) -> PlannedCopy {
    let root = absolute_root(destination_root);
    let available = root.is_dir();
    let mut status = if !available {
        PlanStatus::DestinationUnavailable
    } else if !destination_root.is_absolute() {
        PlanStatus::InvalidPath
    } else {
        PlanStatus::Ready
    };
    if available {
        if let Some(count) = generated.get(&path).copied() {
            let mut next_count = count + 1;
            generated.insert(path.clone(), next_count);
            match policy {
                CollisionPolicy::UniqueSuffix => {
                    let original = path.clone();
                    while generated.contains_key(&path) || path.exists() {
                        path = with_suffix(&original, next_count);
                        next_count += 1;
                    }
                    generated.insert(path.clone(), 1);
                }
                CollisionPolicy::Skip => status = PlanStatus::Skipped,
                _ => status = PlanStatus::DestinationCollision,
            }
        } else if path.exists() && policy == CollisionPolicy::UniqueSuffix {
            let original = path.clone();
            let mut next_count = 2;
            while generated.contains_key(&path) || path.exists() {
                path = with_suffix(&original, next_count);
                next_count += 1;
            }
            generated.insert(path.clone(), 1);
        } else if path.exists() {
            generated.insert(path.clone(), 1);
            status = if path.metadata().map(|m| m.len()).ok() == Some(input.item.size)
                && crate::quick_source(source, &input.item.locator, input.item.size)
                    .ok()
                    .and_then(|a| crate::quick_path(&path).ok().map(|b| a == b))
                    .unwrap_or(false)
            {
                PlanStatus::ExistingIdentical
            } else {
                match policy {
                    CollisionPolicy::Skip => PlanStatus::Skipped,
                    _ => PlanStatus::DestinationCollision,
                }
            };
        } else {
            generated.insert(path.clone(), 1);
        }
    }
    if status == PlanStatus::Ready {
        roots
            .entry(root.clone())
            .and_modify(|value| value.0 = value.0.saturating_add(input.item.size))
            .or_insert((input.item.size, available));
    }
    let temporary_destination = path.with_file_name(format!(
        ".captureport-{}.{}.partial",
        path.file_name().unwrap_or_default().to_string_lossy(),
        Uuid::new_v4()
    ));
    PlannedCopy {
        destination_root: root,
        temporary_destination,
        final_destination: path,
        expected_size: input.item.size,
        required,
        status,
    }
}

fn absolute_root(root: &Path) -> PathBuf {
    fs::canonicalize(root).unwrap_or_else(|_| {
        if root.is_absolute() {
            root.to_path_buf()
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(root)
        }
    })
}

fn with_suffix(path: &Path, number: usize) -> PathBuf {
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let ext = path.extension().and_then(|s| s.to_str());
    let name = match ext {
        Some(ext) => format!("{stem}-{number}.{ext}"),
        None => format!("{stem}-{number}"),
    };
    path.with_file_name(name)
}

fn split_name(name: &str) -> (&str, &str) {
    match name.rsplit_once('.') {
        Some((stem, extension)) => (stem, extension),
        None => (name, ""),
    }
}

fn free_space(path: &Path) -> Option<u64> {
    nix::sys::statvfs::statvfs(path).ok().map(|stats| {
        stats
            .blocks_available()
            .saturating_mul(stats.fragment_size())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use captureport_core::{FakeMediaSource, MediaId};
    use chrono::TimeZone;

    fn input(id: u64, name: &str, time: DateTime<FixedOffset>) -> PlanInput {
        let item = MediaItem::new(MediaId(id), captureport_core::SourceId(1), name, 1024);
        PlanInput {
            item,
            capture_time: time,
            session_name: None,
        }
    }
    #[test]
    fn folder_exclusions_also_filter_previously_selected_media() {
        let dir = tempfile::tempdir().unwrap();
        let mut preset = ImportPreset::everyday(dir.path());
        preset.media_rules.exclude_folders = vec!["Private".into()];
        let time = FixedOffset::east_opt(0)
            .unwrap()
            .with_ymd_and_hms(2026, 1, 1, 12, 0, 0)
            .unwrap();
        let selected = vec![
            input(1, "DCIM/Private/a.jpg", time),
            input(2, "DCIM/Public/b.jpg", time),
        ];
        let plan = ImportPlanner::build_with_explicit_members(
            &FakeMediaSource::new(2),
            selected,
            &preset,
            &HashSet::from([MediaId(1)]),
        );
        assert_eq!(plan.items.len(), 1);
        assert_eq!(plan.items[0].media_id, MediaId(2));
        assert_eq!(plan.items[0].sequence, 1);
    }

    #[test]
    fn canonical_order_and_time_gap_sessions_are_stable() {
        let dir = tempfile::tempdir().unwrap();
        let mut preset = ImportPreset::organized(dir.path());
        preset.grouping = Grouping::TimeGap {
            threshold_minutes: 30,
        };
        let zone = FixedOffset::east_opt(0).unwrap();
        let source = FakeMediaSource::new(2);
        let selected = vec![
            input(
                2,
                "b.JPG",
                zone.with_ymd_and_hms(2026, 1, 1, 12, 40, 0).unwrap(),
            ),
            input(
                1,
                "a.JPG",
                zone.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap(),
            ),
        ];
        let plan = ImportPlanner::build(&source, selected, &preset);
        assert_eq!(plan.items[0].source_name, "a.JPG");
        assert_eq!(plan.items[0].session, 1);
        assert_eq!(plan.items[1].session, 2);
        assert_ne!(
            plan.items[0].copies[0].final_destination,
            plan.items[1].copies[0].final_destination
        );
    }

    #[test]
    fn session_name_reaches_destination_templates() {
        let dir = tempfile::tempdir().unwrap();
        let mut preset = ImportPreset::organized(dir.path());
        preset.photo.root = dir.path().into();
        preset.photo.folder_template = "{session_name}/{session:02}".into();
        preset.filename_template = "{original_name}".into();
        let zone = FixedOffset::east_opt(0).unwrap();
        let mut selected = input(
            1,
            "a.JPG",
            zone.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap(),
        );
        selected.session_name = Some("Iceland trip".into());
        let source = FakeMediaSource::new(1);
        let plan = ImportPlanner::build(&source, vec![selected], &preset);
        assert_eq!(plan.items[0].status, PlanStatus::Ready);
        assert_eq!(
            plan.items[0].copies[0].final_destination,
            dir.path().join("Iceland trip/01/a.JPG")
        );
    }

    #[test]
    fn embedded_metadata_drives_primary_and_backup_destination_templates() {
        let dir = tempfile::tempdir().unwrap();
        let backup = tempfile::tempdir().unwrap();
        let mut preset = ImportPreset::organized(dir.path());
        preset.photo.root = dir.path().into();
        preset.photo.folder_template =
            "{date:%Y-%m-%d}/{camera_make}/{camera_model}/{lens}/{dimensions}".into();
        preset.filename_template = "{camera}_{camera_serial}_{sequence:04}.{extension}".into();
        preset.backup = Some(crate::BackupRule {
            photo: crate::DestinationRule {
                root: backup.path().into(),
                folder_template: preset.photo.folder_template.clone(),
            },
            video: preset.video.clone(),
            required: true,
        });
        let zone = FixedOffset::east_opt(0).unwrap();
        let mut selected = input(
            1,
            "a.JPG",
            zone.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap(),
        );
        selected.item.metadata = MetadataState::Ready(captureport_core::MediaMetadata {
            camera_make: Some("Sony".into()),
            camera_model: Some("A7C/II".into()),
            camera_serial: Some("123".into()),
            lens: Some("FE 24/70".into()),
            width: Some(6000),
            height: Some(4000),
            ..Default::default()
        });
        let source = FakeMediaSource::new(1);
        let plan = ImportPlanner::build(&source, vec![selected], &preset);
        assert_eq!(plan.items[0].status, PlanStatus::Ready);
        let relative = "2026-01-01/Sony/A7C_II/FE 24_70/6000x4000/A7C_II_123_0001.JPG";
        assert_eq!(
            plan.items[0].copies[0].final_destination,
            dir.path().join(relative)
        );
        assert_eq!(
            plan.items[0].copies[1].final_destination,
            backup.path().join(relative)
        );
    }

    #[test]
    fn time_gap_splits_only_after_threshold() {
        let zone = FixedOffset::east_opt(0).unwrap();
        let start = zone.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap();
        let times = [
            start,
            start + Duration::minutes(30),
            start + Duration::minutes(61),
        ];
        assert_eq!(
            session_numbers(
                &times,
                &Grouping::TimeGap {
                    threshold_minutes: 30
                }
            ),
            vec![1, 1, 2]
        );
    }

    #[test]
    fn time_gap_folders_are_controlled_by_primary_and_backup_templates() {
        let primary = tempfile::tempdir().unwrap();
        let backup = tempfile::tempdir().unwrap();
        let mut preset = ImportPreset::everyday(primary.path());
        preset.photo.root = primary.path().into();
        preset.photo.folder_template = "{year}/{date}".into();
        preset.grouping = Grouping::TimeGap {
            threshold_minutes: 30,
        };
        preset.backup = Some(crate::BackupRule {
            photo: crate::DestinationRule {
                root: backup.path().into(),
                folder_template: String::new(),
            },
            video: preset.video.clone(),
            required: true,
        });
        let start = FixedOffset::east_opt(0)
            .unwrap()
            .with_ymd_and_hms(2022, 12, 24, 12, 0, 0)
            .unwrap();
        let source = FakeMediaSource::new(2);
        let selected = vec![
            input(1, "a.JPG", start),
            input(2, "b.JPG", start + Duration::minutes(31)),
        ];
        let plan = ImportPlanner::build(&source, selected.clone(), &preset);
        for (index, name) in ["a.JPG", "b.JPG"].iter().enumerate() {
            let item = &plan.items[index];
            assert_eq!(item.status, PlanStatus::Ready);
            assert_eq!(item.session, index as u32 + 1);
            assert_eq!(item.sequence, 1);
            assert_eq!(
                item.copies[0].final_destination,
                primary.path().join("2022/20221224").join(name)
            );
            assert_eq!(item.copies[1].final_destination, backup.path().join(name));
        }

        preset.photo.folder_template = "{year}/{date}_{session}".into();
        let plan = ImportPlanner::build(&source, selected, &preset);
        assert_eq!(
            plan.items[0].copies[0].final_destination,
            primary.path().join("2022/20221224_01/a.JPG")
        );
        assert_eq!(
            plan.items[1].copies[0].final_destination,
            primary.path().join("2022/20221224_02/b.JPG")
        );
    }
    #[test]
    fn invalid_template_and_unknown_media_are_reported() {
        let dir = tempfile::tempdir().unwrap();
        let mut preset = ImportPreset::everyday(dir.path());
        preset.filename_template = "{unknown}".into();
        let source = FakeMediaSource::new(1);
        let zone = FixedOffset::east_opt(0).unwrap();
        let plan = ImportPlanner::build(
            &source,
            vec![input(1, "x.ARW", zone.timestamp_opt(0, 0).unwrap())],
            &preset,
        );
        assert_eq!(plan.items[0].status, PlanStatus::InvalidPath);
        let plan = ImportPlanner::build(
            &source,
            vec![input(1, "x.bin", zone.timestamp_opt(0, 0).unwrap())],
            &ImportPreset::everyday(dir.path()),
        );
        assert_eq!(plan.items[0].status, PlanStatus::UnsupportedSource);
    }
    #[test]
    fn week_grouping_uses_iso_week_across_calendar_year() {
        let dir = tempfile::tempdir().unwrap();
        let mut preset = ImportPreset::everyday(dir.path());
        preset.photo.root = dir.path().to_path_buf();
        preset.photo.folder_template.clear();
        preset.grouping = Grouping::Week;
        let zone = FixedOffset::east_opt(0).unwrap();
        let source = FakeMediaSource::new(2);
        let plan = ImportPlanner::build(
            &source,
            vec![
                input(
                    1,
                    "a.JPG",
                    zone.with_ymd_and_hms(2026, 12, 31, 12, 0, 0).unwrap(),
                ),
                input(
                    2,
                    "b.JPG",
                    zone.with_ymd_and_hms(2027, 1, 1, 12, 0, 0).unwrap(),
                ),
            ],
            &preset,
        );
        assert_eq!(plan.items[0].session, plan.items[1].session);
        assert!(
            plan.items[0].copies[0]
                .final_destination
                .to_string_lossy()
                .contains("2026-W53")
        );
    }

    #[test]
    fn assumed_timezone_reinterprets_capture_time_and_destination() {
        let dir = tempfile::tempdir().unwrap();
        let mut preset = ImportPreset::everyday(dir.path());
        preset.photo.root = dir.path().to_path_buf();
        preset.photo.folder_template = "{year}-{month}-{day}".into();
        preset.time_correction.assumed_utc_offset_seconds = Some(2 * 3600);
        let zone = FixedOffset::east_opt(0).unwrap();
        let source = FakeMediaSource::new(1);
        let plan = ImportPlanner::build(
            &source,
            vec![input(
                1,
                "a.JPG",
                zone.with_ymd_and_hms(2024, 5, 1, 10, 0, 0).unwrap(),
            )],
            &preset,
        );
        assert_eq!(
            plan.items[0].effective_time.offset().local_minus_utc(),
            7200
        );
        assert_eq!(
            plan.items[0].effective_time.naive_local().to_string(),
            "2024-05-01 10:00:00"
        );
        assert!(
            plan.items[0].copies[0]
                .final_destination
                .to_string_lossy()
                .contains("2024-05-01")
        );
    }

    #[test]
    fn raw_jpeg_pair_shares_a_sequence_and_the_jpeg_sits_beside_the_raw() {
        let dir = tempfile::tempdir().unwrap();
        let mut preset = ImportPreset::everyday(dir.path());
        preset.photo.root = dir.path().to_path_buf();
        preset.photo.folder_template.clear();
        preset.filename_template = "{date}_{sequence:04}.{extension}".into();
        let zone = FixedOffset::east_opt(0).unwrap();
        let time = zone.with_ymd_and_hms(2024, 5, 1, 10, 0, 0).unwrap();
        let source = FakeMediaSource::new(2);
        let plan = ImportPlanner::build(
            &source,
            vec![input(1, "PAIR.ARW", time), input(2, "PAIR.JPG", time)],
            &preset,
        );
        assert_eq!(plan.items.len(), 2);
        let raw = plan
            .items
            .iter()
            .find(|item| item.source_name.ends_with("ARW"))
            .unwrap();
        let jpeg = plan
            .items
            .iter()
            .find(|item| item.source_name.ends_with("JPG"))
            .unwrap();
        assert_eq!(raw.sequence, jpeg.sequence);
        assert_eq!(
            jpeg.copies[0].final_destination,
            raw.copies[0].final_destination.with_extension("jpg")
        );
        assert_eq!(raw.copies[0].final_destination.extension().unwrap(), "ARW");
    }
}

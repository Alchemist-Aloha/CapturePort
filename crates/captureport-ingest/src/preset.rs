use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use captureport_core::{MediaType, classify_path};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum VerificationMode {
    Fast,
    #[default]
    Standard,
    Strict,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum CollisionPolicy {
    #[default]
    Stop,
    Skip,
    UniqueSuffix,
    VerifyIdentical,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum BundlePolicy {
    #[default]
    KeepAll,
    RawOnly,
    JpegOnly,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum Grouping {
    #[default]
    None,
    Day,
    Week,
    Month,
    Year,
    TimeGap {
        threshold_minutes: u32,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DestinationRule {
    pub root: PathBuf,
    pub folder_template: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BackupRule {
    pub photo: DestinationRule,
    pub video: DestinationRule,
    pub required: bool,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct TimeCorrection {
    pub offset_seconds: i64,
    pub assumed_utc_offset_seconds: Option<i32>,
}

/// User-tunable rules deciding which source files belong in the library.
///
/// Extensions match case-insensitively and may be written with or without a
/// leading dot. `include_photo`/`include_video` force odd extensions to be
/// imported as stills or video, `exclude` drops extensions outright, and
/// `ignore` drops whole media types (`raw`, `photo`, `video`, `sidecar`,
/// `unknown`).
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MediaRules {
    pub include_photo: Vec<String>,
    pub include_video: Vec<String>,
    pub exclude: Vec<String>,
    pub ignore: Vec<String>,
}

impl Default for MediaRules {
    fn default() -> Self {
        Self {
            include_photo: Vec::new(),
            include_video: Vec::new(),
            exclude: Vec::new(),
            ignore: vec!["sidecar".into(), "unknown".into()],
        }
    }
}

fn matches_extension(entries: &[String], extension: &str) -> bool {
    entries.iter().any(|entry| {
        entry
            .trim()
            .trim_start_matches('.')
            .eq_ignore_ascii_case(extension)
    })
}

impl MediaRules {
    /// Classify a path, returning `None` when the rules exclude the file.
    pub fn classify(&self, path: impl AsRef<Path>) -> Option<MediaType> {
        let extension = path
            .as_ref()
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if matches_extension(&self.exclude, &extension) {
            return None;
        }
        let media_type = if matches_extension(&self.include_photo, &extension) {
            MediaType::Jpeg
        } else if matches_extension(&self.include_video, &extension) {
            MediaType::Video
        } else {
            classify_path(path)
        };
        let kind = match media_type {
            MediaType::Raw => "raw",
            MediaType::Jpeg | MediaType::Heif | MediaType::Png | MediaType::Tiff => "photo",
            MediaType::Video => "video",
            MediaType::Sidecar => "sidecar",
            MediaType::Unknown => "unknown",
        };
        if self
            .ignore
            .iter()
            .any(|entry| entry.trim().eq_ignore_ascii_case(kind))
        {
            return None;
        }
        Some(media_type)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ImportPreset {
    pub name: String,
    pub photo: DestinationRule,
    pub video: DestinationRule,
    pub filename_template: String,
    pub verification: VerificationMode,
    pub collision: CollisionPolicy,
    pub bundle_policy: BundlePolicy,
    pub grouping: Grouping,
    pub time_correction: TimeCorrection,
    #[serde(default)]
    pub media_rules: MediaRules,
    pub backup: Option<BackupRule>,
}

impl ImportPreset {
    pub fn everyday(home: &Path) -> Self {
        Self {
            name: "Everyday".into(),
            photo: DestinationRule {
                root: home.join("Pictures"),
                folder_template: "{year}/{date}".into(),
            },
            video: DestinationRule {
                root: home.join("Videos"),
                folder_template: "{year}/{date}".into(),
            },
            filename_template: "{original_name}".into(),
            verification: VerificationMode::Standard,
            collision: CollisionPolicy::Stop,
            bundle_policy: BundlePolicy::KeepAll,
            grouping: Grouping::None,
            time_correction: TimeCorrection::default(),
            media_rules: MediaRules::default(),
            backup: None,
        }
    }

    pub fn organized(home: &Path) -> Self {
        Self {
            name: "Organized".into(),
            filename_template: "{date}_{time}_{sequence:04}.{extension}".into(),
            ..Self::everyday(home)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_round_trip_as_versionable_json() {
        let preset = ImportPreset::everyday(Path::new("/home/test"));
        assert_eq!(
            preset,
            serde_json::from_str(&serde_json::to_string(&preset).unwrap()).unwrap()
        );
        assert_eq!(preset.verification, VerificationMode::Standard);
        assert_eq!(preset.media_rules, MediaRules::default());
    }

    #[test]
    fn media_rules_exclude_non_media_and_honor_overrides() {
        let rules = MediaRules::default();
        assert_eq!(rules.classify("a/PAIR.ARW"), Some(MediaType::Raw));
        assert_eq!(rules.classify("a/PAIR.JPG"), Some(MediaType::Jpeg));
        assert_eq!(rules.classify("a/CLIP.MP4"), Some(MediaType::Video));
        assert_eq!(rules.classify("a/PAIR.XMP"), None);
        assert_eq!(rules.classify("a/notes.txt"), None);

        let custom = MediaRules {
            include_photo: vec![".dng2".into()],
            include_video: vec!["ODD".into()],
            exclude: vec!["arw".into()],
            ignore: vec!["sidecar".into()],
        };
        assert_eq!(custom.classify("a/SCAN.DNG2"), Some(MediaType::Jpeg));
        assert_eq!(custom.classify("a/CLIP.odd"), Some(MediaType::Video));
        assert_eq!(custom.classify("a/PAIR.ARW"), None);
        assert_eq!(custom.classify("a/PAIR.JPG"), Some(MediaType::Jpeg));
        assert_eq!(custom.classify("a/notes.txt"), Some(MediaType::Unknown));

        let photos_off = MediaRules {
            ignore: vec!["photo".into(), "unknown".into()],
            ..MediaRules::default()
        };
        assert_eq!(photos_off.classify("a/PAIR.JPG"), None);
        assert_eq!(photos_off.classify("a/PAIR.ARW"), Some(MediaType::Raw));
    }
}

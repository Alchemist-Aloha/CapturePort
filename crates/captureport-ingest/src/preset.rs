use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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
    }
}

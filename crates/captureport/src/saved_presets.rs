use crate::{ImportPreset, UiPreferences};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum PresetAction {
    Create,
    Rename,
    Overwrite,
    Delete,
}

/// All Settings values, including choices that are inactive in the import rules.
#[derive(Deserialize, Serialize)]
pub(crate) struct SavedSettings {
    pub(crate) version: u32,
    pub(crate) import: ImportPreset,
    pub(crate) appearance: UiPreferences,
    pub(crate) source_alias: String,
    pub(crate) gap_minutes: String,
    pub(crate) backup_required: bool,
}

impl SavedSettings {
    pub(crate) fn decode(json: &str) -> Result<Self, String> {
        let saved: Self = serde_json::from_str(json).map_err(|error| error.to_string())?;
        if saved.version != 1 {
            return Err(format!("Unsupported preset version {}", saved.version));
        }
        Ok(saved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;

    #[test]
    fn all_settings_round_trip_and_future_versions_are_rejected() {
        let mut import = ImportPreset::organized(std::path::Path::new("/home/test"));
        import.name = "Travel".into();
        import.photo.folder_template = "{year}/{month}".into();
        import.video.folder_template = "{date}".into();
        import.verification = VerificationMode::Strict;
        import.collision = CollisionPolicy::UniqueSuffix;
        import.bundle_policy = BundlePolicy::RawOnly;
        import.grouping = Grouping::TimeGap {
            threshold_minutes: 120,
        };
        import.time_correction.offset_seconds = 300;
        import.time_correction.assumed_utc_offset_seconds = Some(28800);
        import.media_rules.include_photo = vec!["oddphoto".into()];
        import.media_rules.include_video = vec!["oddvideo".into()];
        import.media_rules.exclude = vec!["png".into()];
        import.media_rules.exclude_folders = vec!["DCIM/Private".into()];
        import.media_rules.ignore = vec!["sidecar".into()];
        import.backup = Some(BackupRule {
            photo: DestinationRule {
                root: "/backup/photos".into(),
                folder_template: "{date}".into(),
            },
            video: DestinationRule {
                root: "/backup/videos".into(),
                folder_template: "{date}".into(),
            },
            required: true,
        });
        let mut saved = SavedSettings {
            version: 1,
            import,
            appearance: UiPreferences {
                dark_mode: true,
                scheme: ColorScheme::ALL[2],
                thumbnail_size: 4,
                merge_raw_jpeg: false,
            },
            source_alias: "My phone".into(),
            gap_minutes: "120".into(),
            backup_required: true,
        };
        let json = serde_json::to_string(&saved).unwrap();
        let loaded = SavedSettings::decode(&json).unwrap();
        assert_eq!(loaded.import, saved.import);
        assert_eq!(
            serde_json::to_value(loaded.appearance).unwrap(),
            serde_json::to_value(saved.appearance).unwrap()
        );
        assert_eq!(loaded.source_alias, saved.source_alias);
        assert_eq!(loaded.gap_minutes, saved.gap_minutes);
        assert_eq!(loaded.backup_required, saved.backup_required);
        saved.version = 2;
        assert!(SavedSettings::decode(&serde_json::to_string(&saved).unwrap()).is_err());
        assert!(SavedSettings::decode("not json").is_err());
    }
}

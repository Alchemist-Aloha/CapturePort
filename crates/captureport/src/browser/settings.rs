use crate::*;

impl Browser {
    pub(crate) fn set_language(
        &mut self,
        language: Option<i18n::Language>,
        cx: &mut Context<Self>,
    ) {
        self.ui.language = language;
        i18n::activate(language);
        self.save_ui();
        cx.notify();
    }
    pub(crate) fn show_settings(&mut self, cx: &mut Context<Self>) {
        self.page = Page::Settings;
        cx.notify();
    }
    pub(crate) fn save_source_alias(&mut self, cx: &mut Context<Self>) {
        let Some(source_id) = self.catalog_source_id else {
            self.message = Some(i18n::tr("Open a source before naming it"));
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
                    let _ = tx.send(WorkMessage::SourceAlias(source_id, alias));
                }
                Err(error) => {
                    let _ = tx.send(WorkMessage::Done(Err(error.to_string())));
                }
            },
        );
        cx.notify();
    }
    pub(crate) fn load_saved_preset(&mut self, id: i64, cx: &mut Context<Self>) {
        if self.preset_busy {
            return;
        }
        let Some(record) = self.saved_presets.iter().find(|preset| preset.id == id) else {
            return;
        };
        let saved = match saved_presets::SavedSettings::decode(&record.configuration_json) {
            Ok(saved) => saved,
            Err(error) => {
                self.preset_message = Some(i18n::format(
                    "Could not load preset: {0}",
                    std::slice::from_ref(&error),
                ));
                cx.notify();
                return;
            }
        };
        let name = record.name.clone();
        let replan = self.plan.is_some() || self.planning;
        self.preset = saved.import;
        self.preset.name = name.clone();
        let language = self.ui.language;
        self.ui = saved.appearance;
        self.ui.language = language;
        self.backup_required_choice = saved.backup_required;
        self.settings.sync(&self.preset, cx);
        self.settings
            .preset_name
            .update(cx, |input, cx| input.set_value(name.clone(), cx));
        self.settings
            .source_alias
            .update(cx, |input, cx| input.set_value(saved.source_alias, cx));
        self.settings
            .gap_minutes
            .update(cx, |input, cx| input.set_value(saved.gap_minutes, cx));
        self.selected_preset = Some(id);
        self.preset_confirmation = None;
        self.timezone_menu_open = false;
        self.template_segment_target = None;
        self.invalidate_plan();
        self.rebuild_bundles();
        self.refresh();
        self.pump_thumbnails();
        self.save_preset();
        self.save_ui();
        self.preset_message = Some(i18n::format(
            "Loaded {0}. Save source alias separately; rescan to apply discovery rules.",
            std::slice::from_ref(&name),
        ));
        if replan && !self.scanning {
            self.start_plan(cx);
        }
        cx.notify();
    }

    pub(crate) fn manage_preset(
        &mut self,
        action: saved_presets::PresetAction,
        cx: &mut Context<Self>,
    ) {
        use saved_presets::{PresetAction, SavedSettings};
        if self.preset_busy {
            return;
        }
        let selected = self
            .saved_presets
            .iter()
            .find(|preset| Some(preset.id) == self.selected_preset);
        if action != PresetAction::Create && selected.is_none() {
            self.preset_message = Some(i18n::tr("Choose a saved preset first."));
            cx.notify();
            return;
        }
        if matches!(action, PresetAction::Overwrite | PresetAction::Delete)
            && self.preset_confirmation != Some(action)
        {
            self.preset_confirmation = Some(action);
            cx.notify();
            return;
        }
        self.preset_confirmation = None;
        let name = if action == PresetAction::Overwrite || action == PresetAction::Delete {
            selected.expect("selected preset").name.clone()
        } else {
            self.settings
                .preset_name
                .read(cx)
                .value()
                .trim()
                .to_string()
        };
        if name.is_empty() || name.chars().any(char::is_control) {
            self.preset_message = Some(i18n::tr(
                "Enter a non-empty preset name without control characters.",
            ));
            cx.notify();
            return;
        }
        if matches!(action, PresetAction::Create | PresetAction::Rename)
            && self.saved_presets.iter().any(|preset| {
                preset.name == name
                    && (action == PresetAction::Create || Some(preset.id) != self.selected_preset)
            })
        {
            self.preset_message = Some(i18n::tr(
                "That name already exists. Choose another name or use Overwrite.",
            ));
            cx.notify();
            return;
        }
        let configuration = if matches!(action, PresetAction::Create | PresetAction::Overwrite) {
            let result = self
                .settings
                .read(&self.preset, self.backup_required_choice, cx)
                .and_then(|mut import| {
                    import.name = name.clone();
                    serde_json::to_string(&SavedSettings {
                        version: 1,
                        import,
                        appearance: self.ui,
                        source_alias: self.settings.source_alias.read(cx).value(),
                        gap_minutes: self.settings.gap_minutes.read(cx).value(),
                        backup_required: self.backup_required_choice,
                    })
                    .map_err(|error| error.to_string())
                });
            match result {
                Ok(json) => json,
                Err(error) => {
                    self.preset_message = Some(i18n::format(
                        "Could not save preset: {0}",
                        std::slice::from_ref(&error),
                    ));
                    cx.notify();
                    return;
                }
            }
        } else {
            String::new()
        };
        let id = self.selected_preset;
        let catalog = self.catalog.clone();
        let (tx, rx) = mpsc::channel();
        self.work_receivers.push(rx);
        self.preset_busy = true;
        self.preset_message = Some(i18n::tr("Saving preset changes…"));
        thread::spawn(move || {
            let result = (|| -> Result<_, String> {
                let (selected, message) = match action {
                    PresetAction::Create => {
                        let record = catalog
                            .create_preset(name.clone(), configuration, now())
                            .map_err(|error| error.to_string())?;
                        (
                            Some(record.id),
                            i18n::format("Created {0}.", std::slice::from_ref(&name)),
                        )
                    }
                    PresetAction::Rename => {
                        catalog
                            .rename_preset(id.expect("selected preset"), name.clone(), now())
                            .map_err(|error| error.to_string())?;
                        (
                            id,
                            i18n::format("Renamed preset to {0}.", std::slice::from_ref(&name)),
                        )
                    }
                    PresetAction::Overwrite => {
                        catalog
                            .upsert_preset(id, name.clone(), configuration, now())
                            .map_err(|error| error.to_string())?;
                        (
                            id,
                            i18n::format(
                                "Overwrote {0} with all current settings.",
                                std::slice::from_ref(&name),
                            ),
                        )
                    }
                    PresetAction::Delete => {
                        catalog
                            .delete_preset(id.expect("selected preset"))
                            .map_err(|error| error.to_string())?;
                        (
                            None,
                            i18n::format(
                                "Deleted {0}. Current settings and imported files are unchanged.",
                                std::slice::from_ref(&name),
                            ),
                        )
                    }
                };
                let presets = catalog.list_presets().map_err(|error| error.to_string())?;
                Ok((presets, selected, message))
            })();
            let _ = tx.send(WorkMessage::Presets(result));
        });
        cx.notify();
    }

    pub(crate) fn choose_preset(&mut self, organized: bool, cx: &mut Context<Self>) {
        if self.preset_busy {
            return;
        }
        self.selected_preset = None;
        self.preset_confirmation = None;
        self.preset_message = None;
        self.settings
            .preset_name
            .update(cx, |input, cx| input.set_value(String::new(), cx));
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
        cx.notify();
    }
    pub(crate) fn cycle_verification(&mut self, cx: &mut Context<Self>) {
        self.preset.verification = match self.preset.verification {
            VerificationMode::Fast => VerificationMode::Standard,
            VerificationMode::Standard => VerificationMode::Strict,
            VerificationMode::Strict => VerificationMode::Fast,
        };
        self.invalidate_plan();
        cx.notify();
    }
    pub(crate) fn cycle_bundle_policy(&mut self, cx: &mut Context<Self>) {
        self.preset.bundle_policy = match self.preset.bundle_policy {
            BundlePolicy::KeepAll => BundlePolicy::RawOnly,
            BundlePolicy::RawOnly => BundlePolicy::JpegOnly,
            BundlePolicy::JpegOnly => BundlePolicy::KeepAll,
        };
        self.invalidate_plan();
        cx.notify();
    }
    pub(crate) fn cycle_grouping(&mut self, cx: &mut Context<Self>) {
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
        self.rebuild_gallery_groups();
        cx.notify();
    }
    pub(crate) fn cycle_collision(&mut self, cx: &mut Context<Self>) {
        self.preset.collision = match self.preset.collision {
            CollisionPolicy::Stop => CollisionPolicy::Skip,
            CollisionPolicy::Skip => CollisionPolicy::UniqueSuffix,
            CollisionPolicy::UniqueSuffix => CollisionPolicy::VerifyIdentical,
            CollisionPolicy::VerifyIdentical => CollisionPolicy::Stop,
        };
        self.invalidate_plan();
        cx.notify();
    }
    pub(crate) fn toggle_backup_required(&mut self, cx: &mut Context<Self>) {
        self.backup_required_choice = !self.backup_required_choice;
        self.invalidate_plan();
        cx.notify();
    }
    pub(crate) fn set_timezone_offset(&mut self, offset: Option<i32>, cx: &mut Context<Self>) {
        self.settings.timezone_seconds.update(cx, |input, cx| {
            input.set_value(
                offset.map(|value| value.to_string()).unwrap_or_default(),
                cx,
            )
        });
        self.timezone_menu_open = false;
        self.invalidate_plan();
        cx.notify();
    }
    pub(crate) fn apply_settings(&mut self, cx: &mut Context<Self>) {
        let preset = match self
            .settings
            .read(&self.preset, self.backup_required_choice, cx)
        {
            Ok(preset) => preset,
            Err(error) => {
                self.message = Some(i18n::format("Settings: {0}", std::slice::from_ref(&error)));
                cx.notify();
                return;
            }
        };
        let replan = self.plan.is_some() || self.planning;
        self.preset = preset.clone();
        self.rebuild_gallery_groups();
        self.invalidate_plan();
        self.page = Page::Browser;
        self.message = Some(i18n::tr("Import settings saved; build a new preview"));
        self.save_preset();
        if replan && !self.scanning {
            self.start_plan(cx);
        }
        cx.notify();
    }
    pub(crate) fn clear_thumbnail_cache(&mut self, cx: &mut Context<Self>) {
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
}

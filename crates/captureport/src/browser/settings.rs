use crate::*;

impl Browser {
    pub(crate) fn show_settings(&mut self, cx: &mut Context<Self>) {
        self.page = Page::Settings;
        cx.notify();
    }
    pub(crate) fn save_source_alias(&mut self, cx: &mut Context<Self>) {
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
                    let _ = tx.send(WorkMessage::SourceAlias(source_id, alias));
                }
                Err(error) => {
                    let _ = tx.send(WorkMessage::Done(Err(error.to_string())));
                }
            },
        );
        cx.notify();
    }
    pub(crate) fn choose_preset(&mut self, organized: bool, cx: &mut Context<Self>) {
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
                self.message = Some(format!("Settings: {error}"));
                cx.notify();
                return;
            }
        };
        let replan = self.plan.is_some() || self.planning;
        self.preset = preset.clone();
        self.rebuild_gallery_groups();
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

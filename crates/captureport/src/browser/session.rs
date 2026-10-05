use crate::*;

impl Browser {
    pub(crate) fn save_ui(&mut self) {
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
    pub(crate) fn set_thumbnail_size(&mut self, size: u8, cx: &mut Context<Self>) {
        let size = size.min(4);
        if self.ui.thumbnail_size != size {
            self.ui.thumbnail_size = size;
            self.save_ui();
            cx.notify();
        }
    }
    /// Toggle merged RAW+JPEG display. Separate mode drops the pairing for the
    /// browser, so each file is listed and selected on its own.
    pub(crate) fn set_merge_raw_jpeg(&mut self, merge: bool, cx: &mut Context<Self>) {
        if self.ui.merge_raw_jpeg != merge {
            self.ui.merge_raw_jpeg = merge;
            self.save_ui();
            self.rebuild_bundles();
            self.pump_thumbnails();
            self.refresh();
            cx.notify();
        }
    }
    pub(crate) fn toggle_dark_mode(&mut self, cx: &mut Context<Self>) {
        self.ui.dark_mode = !self.ui.dark_mode;
        self.save_ui();
        cx.notify();
    }
    pub(crate) fn set_scheme(&mut self, scheme: ColorScheme, cx: &mut Context<Self>) {
        if self.ui.scheme != scheme {
            self.ui.scheme = scheme;
            self.save_ui();
            cx.notify();
        }
    }
    pub(crate) fn drain(&mut self, cx: &mut Context<Self>) {
        let mut changed = false;
        while let Ok((manual, result, volumes)) = self.discovery_receiver.try_recv() {
            self.unmounted_cards = volumes.unmounted;
            self.mounted_models = volumes.mounted_models;
            if manual {
                self.discovering = false;
            }
            if !self.discovering {
                match result {
                    Ok((sources, aliases)) => {
                        if manual {
                            self.message = Some(format!(
                                "Device scan complete · {} source(s) found",
                                sources.len() + self.unmounted_cards.len()
                            ));
                        }
                        self.discovered_sources = sources;
                        self.discovery_aliases = aliases;
                    }
                    Err(error) if manual => {
                        self.message = Some(format!("Device scan failed: {error}. Try again."))
                    }
                    Err(_) => {}
                }
                changed = true;
            }
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
                    Ok(ScanMessage::Event(e)) => {
                        if let AppEvent::MediaDiscovered { generation, item } = &*e
                            && generation.0 == self.generation
                        {
                            self.pair_index
                                .note(item.id, item.media_type, &item.source_path);
                        }
                        changed |= self.state.apply_event(*e);
                    }
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
                        self.failed_thumbnails.clear();
                        self.message = Some(format!("Cleared {count} cached thumbnail(s)"));
                        changed = true;
                    }
                    Ok(WorkMessage::Presets(result)) => {
                        self.preset_busy = false;
                        match result {
                            Ok((presets, selected, message)) => {
                                self.saved_presets = presets;
                                self.selected_preset = selected;
                                if let Some(preset) =
                                    self.saved_presets.iter().find(|p| Some(p.id) == selected)
                                {
                                    self.settings.preset_name.update(cx, |input, cx| {
                                        input.set_value(preset.name.clone(), cx)
                                    });
                                }
                                self.preset_message = Some(message);
                            }
                            Err(error) => self.preset_message = Some(format!("Presets: {error}")),
                        }
                        changed = true;
                    }
                    Ok(WorkMessage::SourceAlias(source_id, alias)) => {
                        if self.catalog_source_id == Some(source_id) {
                            self.source_alias = alias;
                            self.message = Some("Source alias saved".into());
                            changed = true;
                        }
                    }
                    Ok(WorkMessage::MarkedImported {
                        generation,
                        media_ids,
                        result,
                    }) => {
                        self.marking_imported = false;
                        if generation == self.state.generation {
                            match result {
                                Ok(()) => {
                                    for id in &media_ids {
                                        self.state.apply_event(AppEvent::ManualImportMarked {
                                            generation,
                                            media_id: *id,
                                        });
                                        self.explicit_bundle_selection.remove(id);
                                    }
                                    self.invalidate_plan();
                                    self.message = Some(format!(
                                        "Marked {} file(s) as imported manually",
                                        media_ids.len()
                                    ));
                                }
                                Err(error) => {
                                    self.message = Some(format!(
                                        "Could not mark selected files as imported: {error}"
                                    ))
                                }
                            }
                        }
                        changed = true;
                    }
                    Ok(WorkMessage::Mounted(result)) => {
                        self.mounting = None;
                        self.message = Some(match result {
                            Ok(name) => {
                                format!("Mounted {name}. Select it in Sources to browse media.")
                            }
                            Err(error) => format!("Could not mount device: {error}"),
                        });
                        let _ = self.discovery_requests.send(());
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
            if let Some(id) = self.thumbnail_keys.remove(&result.key) {
                match result.state {
                    ThumbnailState::Ready => {
                        self.thumbnail_paths.insert(
                            id,
                            self.cache_dir
                                .join("thumbnails")
                                .join(format!("{}.jpg", result.key)),
                        );
                    }
                    ThumbnailState::Placeholder | ThumbnailState::Failed => {
                        self.failed_thumbnails.insert(id);
                    }
                    ThumbnailState::Cancelled => {
                        self.requested_thumbnails.remove(&id);
                    }
                }
                changed = true;
            }
        }
        while let Ok(result) = self.camera_preview_receiver.try_recv() {
            if result.generation == self.generation {
                if let Some(path) = result.cache_path {
                    self.thumbnail_paths.insert(result.id, path);
                } else {
                    self.failed_thumbnails.insert(result.id);
                }
                changed = true;
            }
        }
        if changed {
            self.pump_thumbnails();
            self.refresh();
            cx.notify();
        }
    }
    pub(crate) fn pump_thumbnails(&mut self) {
        // Fill the bounded queue without requiring a visible tile to submit work.
        let ids: Vec<_> = self
            .state
            .items()
            .filter(|item| {
                matches!(item.metadata, captureport_core::MetadataState::Ready(_))
                    && !self.requested_thumbnails.contains(&item.id)
                    // In merged mode a RAW that shares a stem with a JPEG is
                    // rendered from the JPEG, so its own preview is not decoded.
                    && !(self.ui.merge_raw_jpeg && self.pair_index.has_pair(item.id))
            })
            .map(|item| item.id)
            .take(129)
            .collect();
        for id in ids {
            let before = self.requested_thumbnails.len();
            self.request_thumbnail(id);
            if self.requested_thumbnails.len() == before {
                break;
            }
        }
    }
    pub(crate) fn refresh(&mut self) {
        let ready = visible_capture_ids(
            self.state.visible_items().into_iter().map(|item| item.id),
            &self.bundle_owner,
        )
        .into_iter()
        .filter(|id| {
            self.thumbnail_paths.contains_key(&self.preview_id(*id))
                && self.state.item(*id).is_some_and(|item| {
                    matches!(item.metadata, captureport_core::MetadataState::Ready(_))
                })
        })
        .collect::<Vec<_>>();
        append_ready_ids(&mut self.visible_ids, ready);
        self.rebuild_gallery_groups();
    }
    pub(crate) fn rebuild_gallery_groups(&mut self) {
        self.gallery_groups.clear();
        self.gallery_item_names.clear();
        let Grouping::TimeGap { .. } = self.preset.grouping else {
            return;
        };
        let Some(source) = &self.source else {
            return;
        };
        let identity = source.identity();
        let source_key = identity
            .stable_id
            .or(identity.volume_uuid)
            .or(identity.display_name)
            .unwrap_or_else(|| format!("{:?}", identity.source_type));
        let mut items = self
            .state
            .items()
            .filter(|item| !self.bundle_members.contains(&item.id))
            .collect::<Vec<_>>();
        items.sort_by(|a, b| {
            effective_time(capture_time(a), &self.preset)
                .cmp(&effective_time(capture_time(b), &self.preset))
                .then_with(|| a.source_name.cmp(&b.source_name))
                .then_with(|| a.source_path.cmp(&b.source_path))
                .then_with(|| a.id.cmp(&b.id))
        });
        let times = items
            .iter()
            .map(|item| effective_time(capture_time(item), &self.preset))
            .collect::<Vec<_>>();
        let sessions = session_numbers(&times, &self.preset.grouping);
        let visible = self.visible_ids.iter().copied().collect::<HashSet<_>>();
        let mut item_group = Vec::new();
        for (index, item) in items.into_iter().enumerate() {
            if self
                .gallery_groups
                .last()
                .is_none_or(|group| group.session != sessions[index])
            {
                let key = format!("{}|{}|{}", source_key, item.source_path, sessions[index]);
                let title = self
                    .gallery_names
                    .get(&key)
                    .filter(|name| !is_legacy_session_name(name))
                    .cloned()
                    .unwrap_or_else(|| times[index].format("%Y-%m-%d").to_string());
                self.gallery_groups.push(GalleryGroup {
                    key,
                    title,
                    session: sessions[index],
                    ids: Vec::new(),
                });
            }
            item_group.push((
                item.id,
                self.gallery_groups
                    .last()
                    .expect("group exists")
                    .key
                    .clone(),
            ));
            if visible.contains(&item.id) {
                self.gallery_groups
                    .last_mut()
                    .expect("group exists")
                    .ids
                    .push(item.id);
            }
        }
        // Keep the pre-retain titles so items hidden by the current filter still
        // carry the session name the user typed.
        let mut titles = self
            .gallery_groups
            .iter()
            .map(|group| (group.key.clone(), group.title.clone()))
            .collect::<HashMap<_, _>>();
        self.gallery_groups.retain(|group| !group.ids.is_empty());
        suffix_duplicate_titles(&mut self.gallery_groups, |key| {
            self.gallery_names
                .get(key)
                .is_some_and(|name| !is_legacy_session_name(name))
        });
        for group in &self.gallery_groups {
            titles.insert(group.key.clone(), group.title.clone());
        }
        self.gallery_item_names.clear();
        for (id, key) in item_group {
            if let Some(title) = titles.get(&key) {
                self.gallery_item_names.insert(id, title.clone());
            }
        }
    }
    pub(crate) fn set_gap_minutes(&mut self, minutes: u32, cx: &mut Context<Self>) {
        self.preset.grouping = Grouping::TimeGap {
            threshold_minutes: minutes,
        };
        self.settings
            .gap_minutes
            .update(cx, |input, cx| input.set_value(minutes.to_string(), cx));
        self.invalidate_plan();
        self.rebuild_gallery_groups();
        self.save_preset();
        cx.notify();
    }
    pub(crate) fn save_preset(&mut self) {
        let result = serde_json::to_vec_pretty(&self.preset)
            .map_err(|e| e.to_string())
            .and_then(|data| {
                let temporary = self.config_path.with_extension("json.tmp");
                std::fs::write(&temporary, data).map_err(|e| e.to_string())?;
                std::fs::rename(temporary, &self.config_path).map_err(|e| e.to_string())
            });
        if let Err(error) = result {
            self.message = Some(format!("Settings: {error}"));
        }
    }
}

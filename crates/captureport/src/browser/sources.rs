use crate::*;

impl Browser {
    pub(crate) fn scan(&mut self, request: SourceRequest, cx: &mut Context<Self>) {
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
        self.gallery_groups.clear();
        self.gallery_edit_key = None;
        self.bundles.clear();
        self.bundle_members.clear();
        self.bundle_owner.clear();
        self.bundle_preview.clear();
        self.pair_index.clear();
        self.expanded_bundle = None;
        self.explicit_bundle_selection.clear();
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
        self.failed_thumbnails.clear();
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
        let media_rules = self.preset.media_rules.clone();
        thread::spawn(move || scan_source(request, scan, media_rules, tx, cat));
        cx.notify();
    }
    pub(crate) fn open_folder(&mut self, _: &OpenFolder, _: &mut Window, cx: &mut Context<Self>) {
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
    pub(crate) fn open_demo(&mut self, _: &OpenDemo, _: &mut Window, cx: &mut Context<Self>) {
        self.scan(
            SourceRequest::Demo(FakeSourceBuilder::new().scenario(FakeSourceScenario::LargeCard)),
            cx,
        )
    }
    pub(crate) fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        if self.page != Page::Browser {
            return;
        }
        let ids = self
            .state
            .visible_items()
            .into_iter()
            .map(|item| item.id)
            .collect::<Vec<_>>();
        self.state.select_all(true);
        self.explicit_bundle_selection.extend(ids);
        self.invalidate_plan();
        self.page = Page::Browser;
        cx.notify()
    }
    pub(crate) fn select_new(&mut self, _: &SelectAllNew, _: &mut Window, cx: &mut Context<Self>) {
        if self.page != Page::Browser {
            return;
        }
        let ids = self
            .state
            .visible_items()
            .into_iter()
            .filter(|item| item.import_status == captureport_core::ImportStatus::New)
            .map(|item| item.id)
            .collect::<Vec<_>>();
        self.state.select_all_new();
        self.explicit_bundle_selection.extend(ids);
        self.invalidate_plan();
        self.page = Page::Browser;
        cx.notify()
    }
    pub(crate) fn select_none(&mut self, _: &SelectNone, _: &mut Window, cx: &mut Context<Self>) {
        if self.page != Page::Browser {
            return;
        }
        self.state.select_all(false);
        self.explicit_bundle_selection
            .retain(|id| self.state.is_selected(*id));
        self.invalidate_plan();
        self.page = Page::Browser;
        cx.notify()
    }
    pub(crate) fn can_mark_selected_imported(&self) -> bool {
        !self.scanning
            && !self.importing
            && !self.planning
            && !self.marking_imported
            && self.state.items().any(|item| {
                self.state.is_selected(item.id)
                    && item.import_status != captureport_core::ImportStatus::Imported
            })
    }
    pub(crate) fn mark_selected_imported(
        &mut self,
        _: &MarkSelectedImported,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.can_mark_selected_imported() {
            return;
        }
        let selected = match manual_mark_selection(&self.state, &self.catalog_media_ids) {
            Ok(selected) => selected,
            Err(error) => {
                self.message = Some(error);
                cx.notify();
                return;
            }
        };
        let generation = self.state.generation;
        let catalog = self.catalog.clone();
        let (tx, rx) = mpsc::channel();
        self.work_receivers.push(rx);
        self.marking_imported = true;
        self.invalidate_plan();
        self.message = Some(format!(
            "Marking {} selected file(s) as imported…",
            selected.len()
        ));
        thread::spawn(move || {
            let catalog_ids = selected.iter().map(|(_, catalog_id)| *catalog_id).collect();
            let result = catalog
                .mark_manually_imported(catalog_ids, now())
                .map_err(|error| error.to_string());
            let _ = tx.send(WorkMessage::MarkedImported {
                generation,
                media_ids: selected.into_iter().map(|(id, _)| id).collect(),
                result,
            });
        });
        cx.notify();
    }
    pub(crate) fn filter(&mut self, f: MediaFilter, cx: &mut Context<Self>) {
        self.state.filter = f;
        self.invalidate_plan();
        self.page = Page::Browser;
        self.refresh();
        cx.notify()
    }
    pub(crate) fn sort(&mut self, s: MediaSort, cx: &mut Context<Self>) {
        self.state.sort = s;
        self.refresh();
        cx.notify()
    }
    pub(crate) fn discover_sources(
        &mut self,
        _: &DiscoverSources,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.discovering {
            return;
        }
        self.discovering = true;
        self.message = Some("Scanning for cameras and cards…".into());
        if self.discovery_requests.send(()).is_err() {
            self.discovering = false;
            self.message = Some("Device scanner stopped. Restart CapturePort to try again.".into());
        }
        cx.notify()
    }
    pub(crate) fn mount_card(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.mounting.is_some() {
            return;
        }
        let Some(card) = self.unmounted_cards.get(index).cloned() else {
            return;
        };
        self.mounting = Some(card.path.clone());
        self.message = Some(format!("Mounting {}…", card.label));
        let (tx, rx) = mpsc::channel();
        self.work_receivers.push(rx);
        thread::spawn(move || {
            let result = (|| -> Result<String, String> {
                if !removable_volumes()?.unmounted.contains(&card) {
                    return Err("Device changed or is no longer available. Refresh devices.".into());
                }
                let output = Command::new("udisksctl")
                    .arg("mount")
                    .arg("-b")
                    .arg(&card.path)
                    .output()
                    .map_err(|e| format!("udisksctl: {e}"))?;
                if !output.status.success() {
                    return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
                }
                Ok(card.label)
            })();
            let _ = tx.send(WorkMessage::Mounted(result));
        });
        cx.notify();
    }
    pub(crate) fn open_discovered(&mut self, index: usize, cx: &mut Context<Self>) {
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

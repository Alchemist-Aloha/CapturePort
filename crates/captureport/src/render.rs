use gpui::{AnyElement, ObjectFit, Rgba, StyledImage};

#[derive(Clone, Copy)]
struct Palette {
    canvas: Rgba,
    panel: Rgba,
    card: Rgba,
    border: Rgba,
    muted: Rgba,
    placeholder: Rgba,
    selected: Rgba,
    accent: Rgba,
    danger: Rgba,
    text: Rgba,
}
impl Palette {
    fn new(dark: bool) -> Self {
        if dark {
            Self {
                canvas: rgb(0x101816), panel: rgb(0x17221f), card: rgb(0x20302b),
                border: rgb(0x344a42), muted: rgb(0xa8c0b4), placeholder: rgb(0x293b34),
                selected: rgb(0x224a3c), accent: rgb(0x7bd6a7), danger: rgb(0x51312e),
                text: rgb(0xe8f1ea),
            }
        } else {
            Self {
                canvas: rgb(0xf7f8f7), panel: rgb(0xf0f3f0), card: rgb(0xffffff),
                border: rgb(0xd5e0d8), muted: rgb(0x52645b), placeholder: rgb(0xe8eeea),
                selected: rgb(0xe9f3ec), accent: rgb(0x44856d), danger: rgb(0xf3e6e2),
                text: rgb(0x202b27),
            }
        }
    }
}

fn thumbnail_columns(available: f32, target: f32) -> usize {
    ((available + 12.) / (target + 12.)).round().max(1.) as usize
}

fn thumbnail_layout(window_width: f32, window_height: f32, sidebar_width: f32, size: u8) -> (usize, f32) {
    let available = (window_width - sidebar_width - 40.).max(1.);
    let target = [172., 210., 250., 300., 360.][size.min(4) as usize];
    let columns = thumbnail_columns(available, target);
    let card_width = ((available - 12. * (columns - 1) as f32) / columns as f32).max(1.);
    let viewport_height = (window_height - 58. - 42. - 150. - 82.).max(1.);
    let image_height = (card_width - 2.).max(1.).min(target).min(viewport_height);
    (columns, image_height)
}

fn import_status_label(status: &captureport_core::ImportStatus) -> &'static str {
    match status {
        captureport_core::ImportStatus::Checking => "Checking…",
        captureport_core::ImportStatus::New => "New",
        captureport_core::ImportStatus::Imported => "Imported",
        captureport_core::ImportStatus::PossibleDuplicate => "Possible duplicate",
        captureport_core::ImportStatus::Unknown => "Unknown",
    }
}

#[cfg(test)]
mod thumbnail_layout_tests {
    use super::{thumbnail_columns, thumbnail_layout};

    #[test]
    fn grid_reflows_without_reserving_more_than_available_width() {
        for available in [120., 180., 320., 580., 900.] {
            for target in [172., 210., 250., 300., 360.] {
                let columns = thumbnail_columns(available, target);
                assert!(columns >= 1);
                let card_width =
                    (available - (columns - 1) as f32 * 12.) / columns as f32;
                assert!(card_width > 0.);
            }
        }
        assert_eq!(thumbnail_columns(180., 250.), 1);
        assert_eq!(thumbnail_columns(900., 250.), 3);
        assert_eq!(thumbnail_columns(665., 360.), 2);
        assert_eq!(thumbnail_columns(665., 250.), 3);
    }

    #[test]
    fn thumbnails_shrink_with_both_window_dimensions() {
        let (_, normal) = thumbnail_layout(1180., 760., 238., 2);
        let (narrow_columns, narrow) = thumbnail_layout(600., 760., 238., 2);
        let (_, short) = thumbnail_layout(1180., 450., 238., 2);
        assert_eq!(normal, 250.);
        assert_eq!(narrow_columns, 1);
        assert!(narrow <= normal);
        assert!(short < normal);
    }
}

impl Browser {
    fn request_thumbnail(&mut self, id: MediaId) {
        if self.cache_clearing {
            return;
        }
        if self.requested_thumbnails.contains(&id) {
            return;
        }
        let (Some(source_identity), Some(item)) = (&self.state.source, self.state.item(id)) else {
            return;
        };
        if source_identity.source_type == SourceType::Camera {
            let Some(source) = &self.source else {
                return;
            };
            let request = ThumbnailRequest {
                source_id: format!(
                    "{}:{}",
                    source_identity
                        .stable_id
                        .clone()
                        .unwrap_or_else(|| source_identity
                            .display_name
                            .clone()
                            .unwrap_or_default()),
                    self.camera_preview_scan_token
                ),
                relative_path: item.source_path.clone(),
                size: item.size,
                modified_unix: 0,
                path: PathBuf::new(),
                media_type: item.media_type,
                priority: 10,
            };
            let key = ThumbnailPipeline::cache_key(&request);
            let job = CameraPreviewJob {
                generation: self.generation,
                id,
                source: source.clone(),
                locator: item.locator.clone(),
                cache_path: self.cache_dir.join("thumbnails").join(format!("{key}.jpg")),
                cancellation: self.cancellation.clone().unwrap_or_default(),
            };
            if self.camera_preview_sender.try_send(job).is_ok() {
                self.requested_thumbnails.insert(id);
            }
            return;
        }
        let Some(root) = &self.filesystem_root else {
            return;
        };
        let Some(modified_unix) = self.thumbnail_modified.get(&id).copied() else {
            return;
        };
        let request = ThumbnailRequest {
            source_id: source_identity
                .stable_id
                .clone()
                .unwrap_or_else(|| root.display().to_string()),
            relative_path: item.source_path.clone(),
            size: item.size,
            modified_unix,
            path: root.join(&item.source_path),
            media_type: item.media_type,
            priority: 10,
        };
        let key = ThumbnailPipeline::cache_key(&request);
        if self.thumbnails.submit(request).is_ok() {
            self.requested_thumbnails.insert(id);
            self.thumbnail_keys.insert(key, id);
        }
    }

    fn sidebar(&mut self, sidebar_width: f32, cx: &mut Context<Self>) -> AnyElement {
        let p = Palette::new(self.ui.dark_mode);
        let mut panel = div()
            .id("sidebar")
            .w(px(sidebar_width))
            .flex_shrink_0()
            .h_full()
            .p_4()
            .border_r_1()
            .border_color(p.border)
            .bg(p.panel)
            .flex()
            .flex_col()
            .gap_2()
            .overflow_y_scroll()
            .child(
                div()
                    .text_sm()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .flex_shrink_0()
                    .child("Sources"),
            )
            .child(button(
                "Open folder…",
                cx.listener(|t, _, w, c| t.open_folder(&OpenFolder, w, c)),
            ))
            .child(button(
                "Demo · 10,000 items",
                cx.listener(|t, _, w, c| t.open_demo(&OpenDemo, w, c)),
            ))
            .child(button(
                "Discover cameras",
                cx.listener(|t, _, w, c| t.discover_sources(&DiscoverSources, w, c)),
            ));
        for (index, discovered) in self.discovered_sources.iter().enumerate() {
            let name = match discovered {
                captureport_gphoto::discovery::DiscoveredSource::Ptp(camera) => {
                    format!("Camera · {}", camera.model)
                }
                captureport_gphoto::discovery::DiscoveredSource::MountedFilesystem {
                    path,
                    label,
                    ..
                } => format!(
                    "Card · {}",
                    label.clone().unwrap_or_else(|| path.display().to_string())
                ),
            };
            let name = self
                .discovery_aliases
                .get(discovered.stable_id())
                .cloned()
                .unwrap_or(name);
            panel = panel.child(
                div()
                    .id(("source", index))
                    .cursor_pointer()
                    .p_2()
                    .min_h(px(34.))
                    .flex_shrink_0()
                    .rounded_md()
                    .bg(p.card)
                    .on_click(cx.listener(move |t, _, _, c| t.open_discovered(index, c)))
                    .truncate()
                    .child(name),
            );
        }
        let source = self
            .source_alias
            .clone()
            .or_else(|| {
                self.state
                    .source
                    .as_ref()
                    .and_then(|s| s.display_name.clone())
            })
            .unwrap_or_else(|| "No source selected".into());
        panel
            .child(
                div()
                    .mt_2()
                    .flex_shrink_0()
                    .text_sm()
                    .text_color(p.muted)
                    .truncate()
                    .child(source),
            )
            .child(
                div()
                    .mt_3()
                    .flex_shrink_0()
                    .text_sm()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("Workflow"),
            )
            .child(button(
                "Browse media",
                cx.listener(|t, _, _, c| {
                    t.page = Page::Browser;
                    c.notify()
                }),
            ))
            .child(button(
                "Preview / import",
                cx.listener(|t, _, w, c| t.import_selected(&ImportSelected, w, c)),
            ))
            .child(button(
                "Cancel import",
                cx.listener(|t, _, w, c| t.cancel_import(&CancelImport, w, c)),
            ))
            .child(button(
                "History",
                cx.listener(|t, _, w, c| t.history(&ShowHistory, w, c)),
            ))
            .child(button(
                "Recovery",
                cx.listener(|t, _, _, c| t.show_recovery(c)),
            ))
            .child(button(
                "Reconcile library",
                cx.listener(|t, _, w, c| t.reconcile(&ReconcileLibrary, w, c)),
            ))
            .child(button(
                "Cancel library scan",
                cx.listener(|t, _, w, c| t.cancel_reconcile(&CancelReconcile, w, c)),
            ))
            .child(button(
                "Clock correction",
                cx.listener(|t, _, w, c| t.clock(&AdjustClock, w, c)),
            ))
            .child(button(
                "Import settings",
                cx.listener(|t, _, _, c| t.show_settings(c)),
            ))
            .child(button(
                "Clear thumbnail cache",
                cx.listener(|t, _, _, c| t.clear_thumbnail_cache(c)),
            ))
            .child(
                div()
                    .mt_2()
                    .flex_shrink_0()
                    .text_xs()
                    .text_color(p.muted)
                    .child(format!("Preset: {}", self.preset.name)),
            )
            .into_any_element()
    }

    fn browser_panel(&mut self, window: &mut Window, sidebar_width: f32, cx: &mut Context<Self>) -> AnyElement {
        let p = Palette::new(self.ui.dark_mode);
        let visible = self.visible_ids.len();
        let (columns, image_height) = thumbnail_layout(
            f32::from(window.bounds().size.width),
            f32::from(window.bounds().size.height),
            sidebar_width,
            self.ui.thumbnail_size,
        );
        let empty = if self.scanning {
            "Scanning source…"
        } else {
            "Choose a source to browse media"
        };
        div().flex_1().min_w_0().min_h_0().flex().flex_col()
            .child(div().px_5().py_3().flex().flex_wrap().items_center().gap_2().border_b_1().border_color(p.border)
                .child(div().mr_3().font_weight(gpui::FontWeight::SEMIBOLD).child(format!("Media · {visible}")))
                .child(div().flex().flex_wrap().gap_2()
                    .child(chip("All",self.state.filter==MediaFilter::All,cx.listener(|t,_,_,c|t.filter(MediaFilter::All,c))))
                    .child(chip("Photos",self.state.filter==MediaFilter::Photos,cx.listener(|t,_,_,c|t.filter(MediaFilter::Photos,c))))
                    .child(chip("Videos",self.state.filter==MediaFilter::Videos,cx.listener(|t,_,_,c|t.filter(MediaFilter::Videos,c))))
                    .child(chip("New",self.state.filter==MediaFilter::New,cx.listener(|t,_,_,c|t.filter(MediaFilter::New,c))))
                    .child(chip("Imported",self.state.filter==MediaFilter::Imported,cx.listener(|t,_,_,c|t.filter(MediaFilter::Imported,c))))
                    .child(chip("Possible",self.state.filter==MediaFilter::PossibleDuplicates,cx.listener(|t,_,_,c|t.filter(MediaFilter::PossibleDuplicates,c))))))
            .child(div().px_5().py_2().flex().flex_wrap().items_center().gap_2()
                .child(div().flex().flex_wrap().gap_2()
                    .child(button("Select all",cx.listener(|t,_,w,c|t.select_all(&SelectAll,w,c))))
                    .child(button("Select new",cx.listener(|t,_,w,c|t.select_new(&SelectAllNew,w,c))))
                    .child(button("Clear",cx.listener(|t,_,w,c|t.select_none(&SelectNone,w,c)))))
                .child(div().flex().gap_2()
                    .child(chip("Time",self.state.sort==MediaSort::CaptureTime,cx.listener(|t,_,_,c|t.sort(MediaSort::CaptureTime,c))))
                    .child(chip("Name",self.state.sort==MediaSort::Name,cx.listener(|t,_,_,c|t.sort(MediaSort::Name,c))))))
            .child(div().px_5().py_2().flex().flex_wrap().items_center().gap_3()
                .child(div().text_xs().text_color(p.muted).child("Thumbnail size"))
                .child(div().id("thumbnail-size-slider").flex().items_center().gap_1()
                    .children((0..=4).map(|step| {
                        let active = step <= self.ui.thumbnail_size;
                        div().id(("thumbnail-size-step", step as usize)).w(px(30.)).h(px(22.))
                            .flex().items_center().cursor_pointer()
                            .on_click(cx.listener(move |t,_,_,c| t.set_thumbnail_size(step,c)))
                            .on_mouse_move(cx.listener(move |t,e: &gpui::MouseMoveEvent,_,c| {
                                if e.dragging() { t.set_thumbnail_size(step,c); }
                            }))
                            .child(div().w_full().h(px(if active { 6. } else { 4. }))
                                .rounded_full().bg(if active { p.accent } else { p.border }))
                    })))
                .child(div().text_xs().text_color(p.muted).child(format!("{} px", image_height.round() as u32))))
            .child(if visible==0 {
                div().flex_1().flex().items_center().justify_center().text_color(p.muted).child(empty).into_any_element()
            } else {
                uniform_list(("media-grid", columns * 5 + self.ui.thumbnail_size as usize),visible.div_ceil(columns),cx.processor(move |t,range:std::ops::Range<usize>,_,cx|range.map(|row| {
                    let mut view=div().w_full().h(px(image_height + 82.)).flex().gap_3().px_5().py_2();
                    for column in 0..columns {
                        if let Some(&id)=t.visible_ids.get(row*columns+column) {
                            t.request_thumbnail(id);
                            if let Some(item)=t.state.item(id) {
                                let selected=t.state.is_selected(id);
                                let picture = if let Some(path)=t.thumbnail_paths.get(&id) {
                                    div().w_full().h(px(image_height)).overflow_hidden()
                                        .child(img(path.clone()).size_full().object_fit(ObjectFit::Cover))
                                        .into_any_element()
                                } else {
                                    div().w_full().h(px(image_height)).flex().items_center().justify_center().bg(p.placeholder)
                                        .text_color(p.muted).child(if item.media_type==captureport_core::MediaType::Video {"VIDEO"} else {"PHOTO"}).into_any_element()
                                };
                                let metadata = div().w_full().px_2().py_1().min_w_0()
                                    .child(div().text_sm().font_weight(gpui::FontWeight::SEMIBOLD)
                                        .truncate().child(item.source_name.clone()))
                                    .child(div().text_xs().text_color(p.muted).truncate()
                                        .child(format!("{} · {} file(s)", format_size(item.size),
                                            t.bundles.get(&id).map_or(1, Vec::len))))
                                    .child(div().text_xs().text_color(p.muted)
                                        .child(import_status_label(&item.import_status)));
                                view=view.child(div().id(("media",id.0)).flex_1().min_w_0()
                                    .overflow_hidden().rounded_md().border_1()
                                    .border_color(if selected{p.accent}else{p.border})
                                    .bg(if selected{p.selected}else{p.card}).cursor_pointer()
                                    .on_click(cx.listener(move|t,_,_,c|t.toggle_bundle(id,c)))
                                    .child(picture).child(metadata));
                            }
                        } else { view=view.child(div().flex_1()); }
                    }
                    view
                }).collect())).flex_1().into_any_element()
            }).into_any_element()
    }

    fn preview_panel(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let p = Palette::new(self.ui.dark_mode);
        let Some(plan) = &self.plan else {
            return div()
                .flex_1()
                .p_5()
                .child("No preview yet")
                .into_any_element();
        };
        let count = plan.items.len();
        let blocked = plan
            .items
            .iter()
            .filter(|item| !item.status.can_execute())
            .count();
        div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .child(
                div()
                    .px_5()
                    .py_3()
                    .border_b_1()
                    .border_color(p.border)
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child(format!(
                        "Import preview · {count} items · {blocked} blocked"
                    )),
            )
            .child(div().px_5().py_2().text_sm().child(format!(
                "Preset: {} · Verification: {:?}",
                plan.preset_name, plan.verification
            )))
            .child(button(
                "Confirm import",
                cx.listener(|t, _, w, c| t.import_selected(&ImportSelected, w, c)),
            ))
            .child(
                if self.last_import_result.is_some() && self.filesystem_root.is_some() {
                    div()
                        .id("delete-verified-sources")
                        .cursor_pointer()
                        .p_2()
                        .bg(p.danger)
                        .rounded_md()
                        .on_click(cx.listener(|t, _, _, c| t.delete_sources(c)))
                        .child(if self.deletion_armed {
                            "Confirm deleting originals"
                        } else {
                            "Delete verified originals…"
                        })
                        .into_any_element()
                } else {
                    div().into_any_element()
                },
            )
            .child(
                uniform_list(
                    "preview-list",
                    count,
                    cx.processor(move |t, range: std::ops::Range<usize>, _, _| {
                        let Some(plan) = &t.plan else {
                            return Vec::new();
                        };
                        range
                            .map(|index| {
                                let item = &plan.items[index];
                                let destinations = item
                                    .copies
                                    .iter()
                                    .map(|c| {
                                        format!(
                                            "{}{} · {:?}",
                                            c.final_destination.display(),
                                            if c.required { "" } else { " (optional)" },
                                            c.status
                                        )
                                    })
                                    .collect::<Vec<_>>()
                                    .join("\n");
                                div()
                                    .min_h(px(72.))
                                    .px_5()
                                    .py_2()
                                    .border_b_1()
                                    .border_color(p.border)
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .child(format!(
                                                "{} · {:?}",
                                                item.source_name, item.status
                                            )),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(p.muted)
                                            .child(destinations),
                                    )
                            })
                            .collect()
                    }),
                )
                .flex_1(),
            )
            .into_any_element()
    }

    fn history_panel(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let p = Palette::new(self.ui.dark_mode);
        let mut panel = div()
            .id("history-panel")
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .overflow_y_scroll()
            .child(
                div()
                    .px_5()
                    .py_3()
                    .border_b_1()
                    .border_color(p.border)
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child(format!("Import history · {} sessions", self.history.len())),
            );
        for entry in &self.history {
            let session = &entry.detail.session;
            let id = session.id;
            let imported = entry
                .detail
                .imports
                .iter()
                .filter(|item| {
                    matches!(
                        item.status,
                        CatalogImportStatus::Verified | CatalogImportStatus::Completed
                    )
                })
                .count();
            let bytes = entry
                .detail
                .imports
                .iter()
                .filter_map(|item| item.destination_size)
                .sum::<u64>();
            let destination = entry
                .detail
                .imports
                .iter()
                .find_map(|item| {
                    std::path::Path::new(&item.destination_path)
                        .parent()
                        .map(|path| path.display().to_string())
                })
                .unwrap_or_default();
            panel = panel.child(
                div()
                    .id(("history", id as u64))
                    .cursor_pointer()
                    .px_5()
                    .py_3()
                    .border_b_1()
                    .border_color(p.border)
                    .on_click(cx.listener(move |t, _, _, c| t.inspect_session(id, c)))
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child(format!(
                                "{} · {} · {:?}",
                                session.started_at, entry.source_name, session.status
                            )),
                    )
                    .child(div().text_xs().child(format!(
                        "{imported} imported · {} · {destination}",
                        format_size(bytes)
                    ))),
            );
        }
        if let Some(detail) = &self.session_detail {
            panel = panel.child(
                div()
                    .px_5()
                    .py_3()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child(format!(
                        "Session {} · {} copies",
                        detail.session.id,
                        detail.imports.len()
                    )),
            );
            for item in &detail.imports {
                panel = panel.child(
                    div()
                        .px_5()
                        .py_2()
                        .border_b_1()
                        .border_color(p.border)
                        .child(
                            div()
                                .text_sm()
                                .child(format!("{} · {:?}", item.destination_path, item.status)),
                        )
                        .child(div().text_xs().child(format!(
                            "Verification: {} · {}",
                            item.verification_method.as_deref().unwrap_or("pending"),
                            item.error.as_deref().unwrap_or("")
                        ))),
                );
            }
        }
        panel.into_any_element()
    }
    fn recovery_panel(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let p = Palette::new(self.ui.dark_mode);
        let mut panel = div()
            .id("recovery-panel")
            .flex_1()
            .min_w_0()
            .p_5()
            .flex()
            .flex_col()
            .gap_2()
            .overflow_y_scroll()
            .child(
                div()
                    .text_lg()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("Import recovery"),
            )
            .child(div().text_sm().child(format!(
                "{} unfinished session(s) · {} incomplete file(s)",
                self.incomplete_sessions.len(),
                self.partial_files.len()
            )));
        for session in &self.incomplete_sessions {
            panel = panel.child(div().p_3().bg(p.card).rounded_md().child(format!(
                "Session {} · started {} · {} recorded copies",
                session.session.id,
                session.session.started_at,
                session.imports.len()
            )));
        }
        if !self.incomplete_sessions.is_empty() {
            panel = panel.child(button(
                "Review remaining files",
                cx.listener(|t, _, _, c| t.resume_from_current_source(c)),
            ));
        }
        for (index, partial) in self.partial_files.iter().enumerate() {
            panel = panel.child(
                div()
                    .p_3()
                    .bg(p.card)
                    .rounded_md()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(format!(
                        "{} · {}",
                        partial.path.display(),
                        format_size(partial.size)
                    ))
                    .child(
                        div()
                            .id(("clean-partial", index))
                            .cursor_pointer()
                            .p_2()
                            .bg(p.danger)
                            .rounded_md()
                            .on_click(cx.listener(move |t, _, _, c| t.clean_partial(index, c)))
                            .child("Clean incomplete file"),
                    ),
            );
        }
        panel.into_any_element()
    }
    fn settings_panel(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let p = Palette::new(self.ui.dark_mode);
        div()
            .id("settings-panel")
            .flex_1()
            .min_w_0()
            .p_5()
            .flex()
            .flex_col()
            .gap_2()
            .overflow_y_scroll()
            .child(
                div()
                    .text_lg()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("Import settings"),
            )
            .child(settings_field(
                "Current source alias (blank uses device name)",
                self.settings.source_alias.clone(),
            ))
            .child(button(
                "Save source alias",
                cx.listener(|t, _, _, c| t.save_source_alias(c)),
            ))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(button(
                        "Everyday",
                        cx.listener(|t, _, _, c| t.choose_preset(false, c)),
                    ))
                    .child(button(
                        "Organized",
                        cx.listener(|t, _, _, c| t.choose_preset(true, c)),
                    )),
            )
            .child(settings_field(
                "Photo destination",
                self.settings.photo_root.clone(),
            ))
            .child(settings_field(
                "Photo folder template",
                self.settings.photo_folder.clone(),
            ))
            .child(settings_field(
                "Video destination",
                self.settings.video_root.clone(),
            ))
            .child(settings_field(
                "Video folder template",
                self.settings.video_folder.clone(),
            ))
            .child(settings_field(
                "Filename template",
                self.settings.filename.clone(),
            ))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        div()
                            .id("verification")
                            .cursor_pointer()
                            .p_2()
                            .bg(p.placeholder)
                            .child(format!("Verification: {:?}", self.preset.verification))
                            .on_click(cx.listener(|t, _, _, c| t.cycle_verification(c))),
                    )
                    .child(
                        div()
                            .id("bundle-policy")
                            .cursor_pointer()
                            .p_2()
                            .bg(p.placeholder)
                            .child(format!("Bundles: {:?}", self.preset.bundle_policy))
                            .on_click(cx.listener(|t, _, _, c| t.cycle_bundle_policy(c))),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        div()
                            .id("grouping")
                            .cursor_pointer()
                            .p_2()
                            .bg(p.placeholder)
                            .child(format!("Grouping: {:?}", self.preset.grouping))
                            .on_click(cx.listener(|t, _, _, c| t.cycle_grouping(c))),
                    )
                    .child(
                        div()
                            .id("collision")
                            .cursor_pointer()
                            .p_2()
                            .bg(p.placeholder)
                            .child(format!("Collision: {:?}", self.preset.collision))
                            .on_click(cx.listener(|t, _, _, c| t.cycle_collision(c))),
                    ),
            )
            .child(settings_field(
                "Clock correction (seconds)",
                self.settings.clock_seconds.clone(),
            ))
            .child(settings_field(
                "Assumed timezone (seconds east of UTC; blank keeps metadata timezone)",
                self.settings.timezone_seconds.clone(),
            ))
            .child(settings_field(
                "Time-gap session threshold (minutes)",
                self.settings.gap_minutes.clone(),
            ))
            .child(settings_field(
                "Backup photo root (blank disables backup)",
                self.settings.backup_photo_root.clone(),
            ))
            .child(settings_field(
                "Backup video root (blank disables backup)",
                self.settings.backup_video_root.clone(),
            ))
            .child(
                div()
                    .id("backup-required")
                    .cursor_pointer()
                    .p_2()
                    .bg(p.placeholder)
                    .child(format!("Backup required: {}", self.backup_required_choice))
                    .on_click(cx.listener(|t, _, _, c| t.toggle_backup_required(c))),
            )
            .child(button(
                "Apply settings",
                cx.listener(|t, _, _, c| t.apply_settings(c)),
            ))
            .into_any_element()
    }
}
fn settings_field(label: &'static str, input: gpui::Entity<text_input::TextInput>) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(div().text_xs().child(label))
        .child(
            div()
                .border_1()
                .border_color(gpui::rgba(0x8a9a8a88))
                .rounded_md()
                .child(input),
        )
        .into_any_element()
}

impl Render for Browser {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Palette::new(self.ui.dark_mode);
        let sidebar_width = if f32::from(window.bounds().size.width) < 760. { 200. } else { 238. };
        let summary = self.state.selection_summary();
        let status = self.message.clone().unwrap_or_else(|| {
            if self.importing {
                self.progress
                    .as_ref()
                    .map(|p| {
                        format!(
                            "Importing · {}/{} files · {}",
                            p.completed,
                            p.total,
                            format_size(p.bytes_copied)
                        )
                    })
                    .unwrap_or_else(|| "Importing…".into())
            } else if self.scanning {
                "Scanning…".into()
            } else {
                "Ready".into()
            }
        });
        let content = match self.page {
            Page::Browser => self.browser_panel(window, sidebar_width, cx),
            Page::Preview => self.preview_panel(cx),
            Page::History => self.history_panel(cx),
            Page::Recovery => self.recovery_panel(cx),
            Page::Settings => self.settings_panel(cx),
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(p.canvas)
            .text_color(p.text)
            .track_focus(&self.focus_handle(cx))
            .on_action(cx.listener(Self::open_folder))
            .on_action(cx.listener(Self::open_demo))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::select_new))
            .on_action(cx.listener(Self::select_none))
            .on_action(cx.listener(Self::import_selected))
            .on_action(cx.listener(Self::cancel_import))
            .on_action(cx.listener(Self::history))
            .on_action(cx.listener(Self::reconcile))
            .on_action(cx.listener(Self::clock))
            .on_action(cx.listener(Self::cancel_reconcile))
            .on_action(cx.listener(Self::discover_sources))
            .child(
                div()
                    .h(px(58.))
                    .px_6()
                    .flex()
                    .items_center()
                    .justify_between()
                    .bg(rgb(0x193c34))
                    .text_color(rgb(0xffffff))
                    .child(
                        div()
                            .text_lg()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("CapturePort"),
                    )
                    .child(button(
                        if self.ui.dark_mode { "Light mode" } else { "Dark mode" },
                        cx.listener(|t, _, _, c| t.toggle_dark_mode(c)),
                    )),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .child(self.sidebar(sidebar_width, cx))
                    .child(content),
            )
            .child(
                div()
                    .min_h(px(42.))
                    .px_5()
                    .flex()
                    .items_center()
                    .justify_between()
                    .flex_wrap()
                    .border_t_1()
                    .border_color(p.border)
                    .bg(p.card)
                    .text_sm()
                    .child(status)
                    .child(format!(
                        "{} items · {} selected · {}",
                        self.state.len(),
                        summary.count,
                        format_size(summary.bytes)
                    )),
            )
    }
}

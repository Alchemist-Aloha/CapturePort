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
                canvas: rgb(0x0e1813), panel: rgb(0x15251d), card: rgb(0x213329),
                border: rgb(0x3b5543), muted: rgb(0xb8cebd), placeholder: rgb(0x2b4033),
                selected: rgb(0x29543a), accent: rgb(0xa5e0ae), danger: rgb(0x613b35),
                text: rgb(0xf3f3e9),
            }
        } else {
            Self {
                canvas: rgb(0xf7f7f2), panel: rgb(0xeff2e9), card: rgb(0xfffef9),
                border: rgb(0xd3dfd2), muted: rgb(0x4d6355), placeholder: rgb(0xe5ede3),
                selected: rgb(0xdcefdc), accent: rgb(0x276f51), danger: rgb(0xf5e3dc),
                text: rgb(0x1d2f24),
            }
        }
    }
}

fn thumbnail_columns(available: f32, target: f32) -> usize {
    ((available + 12.) / (target + 12.)).round().max(1.) as usize
}

fn thumbnail_layout(window_width: f32, window_height: f32, sidebar_width: f32, size: u8, view_options: bool, bundle_open: bool) -> (usize, f32) {
    let available = (window_width - sidebar_width - 40.).max(1.);
    let target = [172., 210., 250., 300., 360.][size.min(4) as usize];
    let columns = thumbnail_columns(available, target);
    let card_width = ((available - 12. * (columns - 1) as f32) / columns as f32).max(1.);
    let extra_chrome = if view_options { 86. } else { 0. } + if bundle_open { 240. } else { 0. };
    let viewport_height = (window_height - 70. - 42. - 140. - 82. - extra_chrome).max(1.);
    let image_height = (card_width - 2.).max(1.).min(target).min(viewport_height);
    (columns, image_height)
}

fn import_status_label(status: &captureport_core::ImportStatus) -> &'static str {
    match status {
        captureport_core::ImportStatus::Checking => "Checking…",
        captureport_core::ImportStatus::New => "New",
        captureport_core::ImportStatus::Imported => "✓ Imported",
        captureport_core::ImportStatus::PossibleDuplicate => "! Possible duplicate",
        captureport_core::ImportStatus::Unknown => "? Unknown",
    }
}

fn plan_status_label(status: captureport_ingest::PlanStatus) -> &'static str {
    use captureport_ingest::PlanStatus;
    match status {
        PlanStatus::Ready => "Ready",
        PlanStatus::ExistingIdentical => "Already present (identical)",
        PlanStatus::Skipped => "Skipped",
        PlanStatus::DestinationCollision => "Destination collision",
        PlanStatus::InvalidPath => "Invalid path",
        PlanStatus::DestinationUnavailable => "Destination unavailable",
        PlanStatus::InsufficientDiskSpace => "Not enough disk space",
        PlanStatus::UnsupportedSource => "Unsupported source",
    }
}

fn media_type_badge(media_type: captureport_core::MediaType) -> &'static str {
    use captureport_core::MediaType;
    match media_type {
        MediaType::Video => "▶ VIDEO",
        MediaType::Raw => "▣ RAW",
        MediaType::Jpeg => "▣ JPEG",
        MediaType::Heif => "▣ HEIF",
        MediaType::Png => "▣ PNG",
        MediaType::Tiff => "▣ TIFF",
        MediaType::Sidecar => "◇ SIDECAR",
        MediaType::Unknown => "◇ FILE",
    }
}

fn bundle_type_label(
    media_types: impl Iterator<Item = captureport_core::MediaType>,
) -> &'static str {
    use captureport_core::MediaType;
    let (mut raw, mut jpeg, mut video, mut sidecar) = (false, false, false, false);
    for media_type in media_types {
        match media_type {
            MediaType::Raw => raw = true,
            MediaType::Jpeg => jpeg = true,
            MediaType::Video => video = true,
            MediaType::Sidecar => sidecar = true,
            _ => {}
        }
    }
    if raw && jpeg {
        "RAW+JPEG"
    } else if video && sidecar {
        "VIDEO+SIDECAR"
    } else {
        "BUNDLE"
    }
}

fn group_plan_items(items: &[captureport_ingest::PlannedImport]) -> Vec<Vec<usize>> {
    let mut groups = Vec::new();
    let mut index = 0;
    while index < items.len() {
        let mut group = vec![index];
        if let (Some(stem), Some(next)) = (
            items[index]
                .copies
                .first()
                .map(|copy| copy.final_destination.with_extension("")),
            items.get(index + 1),
        ) && next
            .copies
            .first()
            .map(|copy| copy.final_destination.with_extension(""))
            == Some(stem)
        {
            group.push(index + 1);
            index += 1;
        }
        groups.push(group);
        index += 1;
    }
    groups
}

#[cfg(test)]
mod thumbnail_layout_tests {
    use super::{media_type_badge, thumbnail_columns, thumbnail_layout};

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
        let (_, normal) = thumbnail_layout(1180., 760., 238., 2, false, false);
        let (narrow_columns, narrow) = thumbnail_layout(600., 760., 238., 2, false, false);
        let (_, short) = thumbnail_layout(1180., 450., 238., 2, false, false);
        let (_, with_controls) = thumbnail_layout(1180., 450., 238., 2, true, true);
        assert_eq!(normal, 250.);
        assert_eq!(narrow_columns, 1);
        assert!(narrow <= normal);
        assert!(short < normal);
        assert!(with_controls < short);
    }
    #[test]
    fn media_badges_distinguish_video_and_stills() {
        use captureport_core::MediaType;
        assert_eq!(media_type_badge(MediaType::Video), "▶ VIDEO");
        assert_eq!(media_type_badge(MediaType::Raw), "▣ RAW");
        assert_eq!(media_type_badge(MediaType::Jpeg), "▣ JPEG");
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
                    .text_lg()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .flex_shrink_0()
                    .child("Sources"),
            )
            .child(primary_button(
                "Open folder…",
                p,
                self.ui.dark_mode,
                cx.listener(|t, _, w, c| t.open_folder(&OpenFolder, w, c)),
            ))
            .child(button(
                "Refresh devices",
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
                    .id(gpui::ElementId::named_usize("source", index))
                    .cursor_pointer()
                    .p_2()
                    .min_h(px(34.))
                    .flex_shrink_0()
                    .rounded_md()
                    .bg(if self.state.source.as_ref().is_some_and(|source| source.stable_id.as_deref() == Some(discovered.stable_id())) { p.selected } else { p.card })
                    .hover(move |style| style.bg(p.selected))
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
                    .mt_1()
                    .flex_shrink_0()
                    .text_xs()
                    .text_color(p.muted)
                    .truncate()
                    .child(format!("Current: {source}")),
            )
            .child(
                div()
                    .mt_5()
                    .text_lg()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("Workspace"),
            )
            .child(sidebar_nav("Browse media", self.page == Page::Browser, p,
                cx.listener(|t, _, _, c| {
                    t.page = Page::Browser;
                    c.notify()
                }),
            ))
            .child(if self.plan.is_some() {
                sidebar_nav("Review import", self.page == Page::Preview, p,
                    cx.listener(|t, _, _, c| { t.page = Page::Preview; c.notify() }),
                ).into_any_element()
            } else { div().into_any_element() })
            .child(sidebar_nav("History", self.page == Page::History, p,
                cx.listener(|t, _, w, c| t.history(&ShowHistory, w, c)),
            ))
            .child(sidebar_nav("Recovery", self.page == Page::Recovery, p,
                cx.listener(|t, _, _, c| t.show_recovery(c)),
            ))
            .child(sidebar_nav("Import settings", self.page == Page::Settings, p,
                cx.listener(|t, _, _, c| t.show_settings(c)),
            ))
            .child(div().mt_5().text_sm().font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(p.muted).child("Tools"))
            .child(button("Reconcile library",
                cx.listener(|t, _, w, c| t.reconcile(&ReconcileLibrary, w, c)),
            ))
            .child(if self.reconcile_cancellation.is_some() {
                button("Cancel library scan",
                    cx.listener(|t, _, w, c| t.cancel_reconcile(&CancelReconcile, w, c)),
                ).into_any_element()
            } else { div().into_any_element() })
            .child(button(
                "Clear thumbnail cache",
                cx.listener(|t, _, _, c| t.clear_thumbnail_cache(c)),
            ))
            .child(button(
                "Demo · 10,000 items",
                cx.listener(|t, _, w, c| t.open_demo(&OpenDemo, w, c)),
            ))
            .child(if self.importing {
                button("Cancel import", cx.listener(|t, _, w, c| t.cancel_import(&CancelImport, w, c)))
                    .into_any_element()
            } else { div().into_any_element() })
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
            self.show_view_options,
            self.expanded_bundle.is_some(),
        );
        let grouped = matches!(self.preset.grouping, Grouping::TimeGap { .. });
        let rows: Vec<GalleryRow> = if grouped {
            self.gallery_groups.iter().flat_map(|group| {
                group.ids.chunks(columns).enumerate().map(|(index, ids)| GalleryRow {
                    header: (index == 0).then(|| (group.key.clone(), group.title.clone())), ids: ids.to_vec()
                }).collect::<Vec<_>>()
            }).collect()
        } else {
            self.visible_ids.chunks(columns).map(|ids| GalleryRow { header: None, ids: ids.to_vec() }).collect()
        };
        let row_count = rows.len();
        let rows = std::sync::Arc::new(rows);
        let source_name = self.source_alias.clone().or_else(|| self.state.source.as_ref()
            .and_then(|source| source.display_name.clone()))
            .unwrap_or_else(|| "No source open".into());
        div().flex_1().min_w_0().min_h_0().flex().flex_col()
            .child(div().px_5().py_3().flex().flex_wrap().items_center().gap_2().border_b_1().border_color(p.border)
                .child(div().mr_3().min_w_0().flex().flex_col()
                    .child(div().font_weight(gpui::FontWeight::SEMIBOLD).truncate().child(source_name))
                    .child(div().text_xs().text_color(p.muted).child(format!("{visible} captures in view · {} selected overall", self.state.selection_summary().count))))
                .child(div().flex().flex_wrap().gap_2()
                    .child(chip("All",self.state.filter==MediaFilter::All,cx.listener(|t,_,_,c|t.filter(MediaFilter::All,c))))
                    .child(chip("Photos",self.state.filter==MediaFilter::Photos,cx.listener(|t,_,_,c|t.filter(MediaFilter::Photos,c))))
                    .child(chip("Videos",self.state.filter==MediaFilter::Videos,cx.listener(|t,_,_,c|t.filter(MediaFilter::Videos,c))))
                    .child(chip("New",self.state.filter==MediaFilter::New,cx.listener(|t,_,_,c|t.filter(MediaFilter::New,c))))
                    .child(chip("Imported",self.state.filter==MediaFilter::Imported,cx.listener(|t,_,_,c|t.filter(MediaFilter::Imported,c))))
                    .child(chip("Possible",self.state.filter==MediaFilter::PossibleDuplicates,cx.listener(|t,_,_,c|t.filter(MediaFilter::PossibleDuplicates,c))))))
            .child(div().px_5().py_2().flex().flex_wrap().items_center().justify_between().gap_2().border_b_1().border_color(p.border)
                .child(div().flex().flex_wrap().gap_2()
                    .child(button("Select all",cx.listener(|t,_,w,c|t.select_all(&SelectAll,w,c))))
                    .child(button("Select new",cx.listener(|t,_,w,c|t.select_new(&SelectAllNew,w,c))))
                    .child(button("Clear",cx.listener(|t,_,w,c|t.select_none(&SelectNone,w,c)))))
                .child(button(if self.show_view_options { "Hide view options" } else { "View options" },
                    cx.listener(|t,_,_,c| { t.show_view_options = !t.show_view_options; c.notify() }))))
            .child(if self.show_view_options {
                div().px_5().py_3().flex().flex_wrap().items_center().gap_5().border_b_1().border_color(p.border).bg(p.panel)
                    .child(div().flex().items_center().gap_2()
                        .child(div().text_sm().text_color(p.muted).child("Sort by"))
                        .child(chip("Time",self.state.sort==MediaSort::CaptureTime,cx.listener(|t,_,_,c|t.sort(MediaSort::CaptureTime,c))))
                        .child(chip("Name",self.state.sort==MediaSort::Name,cx.listener(|t,_,_,c|t.sort(MediaSort::Name,c)))))
                    .child(div().flex().items_center().gap_2()
                        .child(div().text_sm().text_color(p.muted).child("Thumbnail size"))
                        .child(div().id("thumbnail-size-slider").flex().items_center().gap_1()
                            .children((0..=4).map(|step| {
                                let active = step <= self.ui.thumbnail_size;
                                div().id(gpui::ElementId::named_usize("thumbnail-size-step", step as usize)).w(px(30.)).h(px(28.))
                                    .flex().items_center().cursor_pointer()
                                    .on_click(cx.listener(move |t,_,_,c| t.set_thumbnail_size(step,c)))
                                    .on_mouse_move(cx.listener(move |t,e: &gpui::MouseMoveEvent,_,c| {
                                        if e.dragging() { t.set_thumbnail_size(step,c); }
                                    }))
                                    .child(div().w_full().h(px(if active { 6. } else { 4. }))
                                        .rounded_full().bg(if active { p.accent } else { p.border }))
                            })))
                        .child(div().text_xs().text_color(p.muted).child(format!("{} px", image_height.round() as u32))))
                    .child(div().flex().items_center().gap_2()
                        .child(div().text_sm().text_color(p.muted).child("Session gap"))
                        .child(div().id("gallery-gap-slider").flex().items_center().gap_1()
                            .children([5_u32, 15, 30, 60, 120, 240, 480, 1440].into_iter().map(|minutes| {
                                let active = matches!(self.preset.grouping, Grouping::TimeGap { threshold_minutes } if threshold_minutes >= minutes);
                                div().id(gpui::ElementId::named_usize("gallery-gap-step", minutes as usize)).w(px(24.)).h(px(28.))
                                    .flex().items_center().cursor_pointer()
                                    .on_click(cx.listener(move |t,_,_,c| t.set_gap_minutes(minutes,c)))
                                    .on_mouse_move(cx.listener(move |t,e: &gpui::MouseMoveEvent,_,c| {
                                        if e.dragging() { t.set_gap_minutes(minutes,c); }
                                    }))
                                    .child(div().w_full().h(px(if active { 6. } else { 4. })).rounded_full()
                                        .bg(if active { p.accent } else { p.border }))
                            })))
                        .child(div().text_xs().text_color(p.muted).child(match self.preset.grouping {
                            Grouping::TimeGap { threshold_minutes } => format!("{} min", threshold_minutes),
                            _ => "Off".into(),
                        })))
                    .into_any_element()
            } else { div().into_any_element() })
            .child(self.expanded_bundle_panel(cx))
            .child(if visible==0 {
                div().flex_1().flex().items_center().justify_center().px_5()
                    .child(div().flex().flex_col().items_center().gap_3()
                        .child(div().text_lg().font_weight(gpui::FontWeight::SEMIBOLD)
                            .child(if self.scanning { "Scanning source…" } else if self.state.source.is_none() { "Start with a source" } else if self.state.filter == MediaFilter::All { "No media found in this source" } else { "No media matches this filter" }))
                        .child(div().text_sm().text_color(p.muted)
                            .child(if self.scanning { "Media will appear here as it is found." } else if self.state.source.is_none() { "Open a folder or connect a camera to browse media." } else if self.state.filter == MediaFilter::All { "Try another source or check that it contains supported media." } else { "Choose All to see every capture in this source." }))
                        .child(if self.state.source.is_none() && !self.scanning {
                            div().flex().flex_wrap().justify_center().gap_2()
                                .child(primary_button("Open folder…", p, self.ui.dark_mode,
                                    cx.listener(|t, _, w, c| t.open_folder(&OpenFolder, w, c))))
                                .child(button("Scan for cameras",
                                    cx.listener(|t, _, w, c| t.discover_sources(&DiscoverSources, w, c))))
                                .into_any_element()
                        } else if self.state.filter != MediaFilter::All && !self.scanning {
                            button("Show all media", cx.listener(|t, _, _, c| t.filter(MediaFilter::All, c))).into_any_element()
                        } else { div().into_any_element() }))
                    .into_any_element()
            } else {
                uniform_list(("media-grid", columns * 5 + self.ui.thumbnail_size as usize + if grouped { 100 } else { 0 }),row_count,cx.processor(move |t,range:std::ops::Range<usize>,_,cx|range.map(|row| {
                    let GalleryRow { header, ids } = &rows[row];
                    let mut cards=div().w_full().flex().gap_3();
                    for column in 0..columns {
                        if let Some(&id)=ids.get(column) {
                            t.request_thumbnail(id);
                            if let Some(item)=t.state.item(id) {
                                let members=t.bundles.get(&id);
                                let member_count=members.map_or(1,Vec::len);
                                let selected_count=members.map_or(usize::from(t.state.is_selected(id)), |members| members.iter().filter(|&&member| t.state.is_selected(member)).count());
                                let selected=selected_count==member_count;
                                let partly_selected=selected_count>0;
                                let expanded=t.expanded_bundle==Some(id);
                                let total_size=members.map_or(item.size, |members| members.iter().filter_map(|member| t.state.item(*member)).map(|item| item.size).sum());
                                let badge=members.map_or_else(||media_type_badge(item.media_type),|members|bundle_type_label(members.iter().filter_map(|member|t.state.item(*member)).map(|item|item.media_type)));
                                let timezone=t.preset.time_correction.assumed_utc_offset_seconds;
                                let modified=t.thumbnail_modified.get(&id).copied().map(|seconds| format!(" · {}", format_file_time(seconds,timezone))).unwrap_or_default();
                                let picture = if let Some(path)=t.thumbnail_paths.get(&id) {
                                    div().id(("media-open", id.0)).w_full().h(px(image_height)).overflow_hidden().cursor_pointer()
                                        .on_click(cx.listener(move|t,_,_,c|t.toggle_bundle(id,c)))
                                        .child(img(path.clone()).size_full().object_fit(ObjectFit::Cover))
                                        .into_any_element()
                                } else {
                                    div().id(("media-open", id.0)).w_full().h(px(image_height)).flex().items_center().justify_center().bg(p.placeholder)
                                        .cursor_pointer().on_click(cx.listener(move|t,_,_,c|t.toggle_bundle(id,c)))
                                        .text_color(p.muted).child(if item.media_type==captureport_core::MediaType::Video {"VIDEO"} else {"PHOTO"}).into_any_element()
                                };
                                let metadata = div().w_full().px_2().py_1().min_w_0()
                                    .child(div().id(("media-name", id.0)).min_w_0().cursor_pointer().rounded_sm()
                                        .on_click(cx.listener(move |t, _, _, c| {
                                            c.stop_propagation();
                                            t.toggle_group_selection(id, c);
                                        }))
                                        .hover(move |style| style.bg(p.selected))
                                        .child(div().text_sm().font_weight(gpui::FontWeight::SEMIBOLD)
                                            .truncate().child(item.source_name.clone())))
                                    .child(div().flex().items_center().gap_2().min_w_0()
                                        .child(div().px_1().rounded_sm().bg(p.selected).text_color(p.text)
                                            .text_xs().font_weight(gpui::FontWeight::SEMIBOLD)
                                            .child(badge))
                                        .child(div().text_xs().text_color(p.muted).truncate()
                                            .child(if members.is_some() {
                                                format!("{member_count} files · {}{modified}", format_size(total_size))
                                            } else { format!("{}{modified}", format_size(item.size)) })))
                                    .child(div().id(("media-status", id.0)).text_xs().font_weight(gpui::FontWeight::SEMIBOLD).cursor_pointer()
                                        .on_click(cx.listener(move|t,_,_,c|t.toggle_bundle(id,c)))
                                        .text_color(if partly_selected { p.text } else { p.muted })
                                        .child(if members.is_some() {
                                            format!("{selected_count}/{member_count} selected · {}", if expanded { "Hide files" } else { "View files" })
                                        } else if selected {
                                            format!("Selected · {}", import_status_label(&item.import_status))
                                        } else { import_status_label(&item.import_status).to_string() }));
                                cards=cards.child(div().id(("media",id.0)).flex_1().min_w_0()
                                    .overflow_hidden().rounded_md().border_1()
                                    .border_color(if partly_selected || expanded {p.accent}else{p.border})
                                    .bg(if selected{p.selected}else{p.card})
                                    .child(picture).child(metadata));
                            }
                        } else { cards=cards.child(div().flex_1()); }
                    }
                    let mut view=div().w_full().h(px(image_height + 82. + if grouped { 42. } else { 0. })).flex().flex_col().px_5().py_2();
                    if grouped {
                        let heading = if let Some((key, title)) = header {
                            if t.gallery_edit_key.as_ref() == Some(key) {
                                div().w_full().min_h(px(30.)).flex().items_center().gap_2()
                                    .child(div().flex_1().min_w_0()
                                        .child(themed_input(t.gallery_edit_input.clone(), p, cx)))
                                    .child(button("Save", cx.listener(|t,_,_,c| t.save_gallery_name(c))))
                            } else {
                                let key = key.clone();
                                let name_key = key.clone();
                                div().w_full().min_h(px(30.)).flex().items_center().gap_2()
                                    .child(
                                        div()
                                            .id(gpui::ElementId::Name(format!("gallery-name-{key}").into()))
                                            .flex_1().min_w_0().font_weight(gpui::FontWeight::SEMIBOLD).truncate()
                                            .cursor_pointer().rounded_sm()
                                            .hover(move |style| style.bg(p.selected))
                                            .on_click(cx.listener(move |t, _, _, c| {
                                                t.toggle_gallery_selection(&name_key, c)
                                            }))
                                            .child(title.clone()),
                                    )
                                    .child(button("Rename", cx.listener(move |t,_,_,c| t.edit_gallery(key.clone(),c))))
                            }
                        } else { div().h(px(30.)) };
                        view=view.child(heading);
                    }
                    view.child(cards)
                }).collect())).flex_1().into_any_element()
            }).into_any_element()
    }

    fn expanded_bundle_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(primary) = self.expanded_bundle else {
            return div().into_any_element();
        };
        let Some(members) = self.bundles.get(&primary) else {
            return div().into_any_element();
        };
        let p = Palette::new(self.ui.dark_mode);
        let label = bundle_type_label(
            members
                .iter()
                .filter_map(|member| self.state.item(*member))
                .map(|item| item.media_type),
        );
        let title = self.state.item(primary).map_or_else(
            || "Media group".to_string(),
            |item| format!("{label} · {}", item.source_name),
        );
        let selected_count = members
            .iter()
            .filter(|&&member| self.state.is_selected(member))
            .count();
        let mut panel = div()
            .id("expanded-bundle")
            .flex_shrink_0()
            .max_h(px(240.))
            .overflow_y_scroll()
            .px_5()
            .py_3()
            .border_b_1()
            .border_color(p.border)
            .bg(p.panel)
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(div().id(("group-name", primary.0)).font_weight(gpui::FontWeight::SEMIBOLD).cursor_pointer()
                        .on_click(cx.listener(move |t, _, _, c| t.toggle_group_selection(primary, c)))
                        .child(format!(
                            "{title} · {selected_count}/{} selected",
                            members.len()
                        )))
                    .child(button(
                        "Collapse group",
                        cx.listener(move |t, _, _, c| t.toggle_bundle(primary, c)),
                    )),
            )
            .child(
                div()
                    .my_2()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(button(
                        "Select all in group",
                        cx.listener(move |t, _, _, c| t.select_bundle_members(primary, true, c)),
                    ))
                    .child(button(
                        "Deselect all in group",
                        cx.listener(move |t, _, _, c| t.select_bundle_members(primary, false, c)),
                    )),
            );
        for &id in members {
            let Some(item) = self.state.item(id) else { continue };
            let selected = self.state.is_selected(id);
            let modified = self
                .thumbnail_modified
                .get(&id)
                .copied()
                .map(|seconds| {
                    format!(
                        " · {}",
                        format_file_time(
                            seconds,
                            self.preset.time_correction.assumed_utc_offset_seconds
                        )
                    )
                })
                .unwrap_or_default();
            panel = panel.child(
                div()
                    .id(("bundle-member", id.0))
                    .ml_3()
                    .pl_3()
                    .py_2()
                    .border_l_1()
                    .border_color(p.border)
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .cursor_pointer()
                    .bg(if selected { p.selected } else { p.panel })
                    .hover(move |style| style.bg(p.selected))
                    .on_click(cx.listener(move |t, _, _, c| t.toggle_bundle_member(id, c)))
                    .child(
                        div()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(div().text_sm().truncate().child(item.source_name.clone()))
                            .child(div().text_xs().text_color(p.muted).child(format!(
                                "{} · {} · {}{modified}",
                                media_type_badge(item.media_type),
                                format_size(item.size),
                                import_status_label(&item.import_status)
                            ))),
                    )
                    .child(div().text_sm().font_weight(gpui::FontWeight::SEMIBOLD).child(
                        if selected { "Selected" } else { "Select" },
                    )),
            );
        }
        panel.into_any_element()
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
        let groups = group_plan_items(&plan.items);
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
            .child(div().px_5().py_3().border_b_1().border_color(p.border)
                .child(div().flex().flex_wrap().items_center().justify_between().gap_2()
                    .child(div().text_lg().font_weight(gpui::FontWeight::SEMIBOLD)
                        .child(format!("Review import · {count} files")))
                    .child(button("Back to media", cx.listener(|t, _, _, c| { t.page = Page::Browser; c.notify() }))))
                .child(div().text_sm().text_color(p.muted)
                    .child(if blocked > 0 { format!("{blocked} blocked · Review the affected paths below") } else { "Check each final destination before copying.".into() })))
            .child(div().px_5().py_3().flex().flex_wrap().items_center().justify_between().gap_3().border_b_1().border_color(p.border).bg(p.panel)
                .child(div().text_sm().child(format!(
                    "{} can proceed · {} blocked · {} preset · {:?} verification",
                    count.saturating_sub(blocked), blocked, plan.preset_name, plan.verification
                )))
                .child(if blocked == 0 && count > 0 && !self.importing && self.last_import_result.is_none() {
                    primary_button("Confirm import", p, self.ui.dark_mode,
                        cx.listener(|t, _, w, c| t.import_selected(&ImportSelected, w, c)))
                        .into_any_element()
                } else if blocked > 0 {
                    div().flex().items_center().gap_2()
                        .child(div().px_3().py_2().rounded_md().bg(p.border).text_sm().text_color(p.muted).child("Import blocked"))
                        .child(button("Edit import settings", cx.listener(|t, _, _, c| t.show_settings(c))))
                        .into_any_element()
                } else { div().into_any_element() }))
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
                    groups.len(),
                    cx.processor(move |t, range: std::ops::Range<usize>, _, _| {
                        let Some(plan) = &t.plan else {
                            return Vec::new();
                        };
                        range
                            .map(|index| {
                                let items: Vec<_> = groups[index]
                                    .iter()
                                    .map(|&item| &plan.items[item])
                                    .collect();
                                let names = items
                                    .iter()
                                    .map(|item| item.source_name.clone())
                                    .collect::<Vec<_>>()
                                    .join(" + ");
                                let status = items
                                    .iter()
                                    .map(|item| item.status)
                                    .find(|status| !status.can_execute())
                                    .unwrap_or(items[0].status);
                                let modified = items
                                    .iter()
                                    .find_map(|item| t.thumbnail_modified.get(&item.media_id))
                                    .copied()
                                    .map(|seconds| {
                                        format!(
                                            " · Modified {}",
                                            format_file_time(
                                                seconds,
                                                t.preset
                                                    .time_correction
                                                    .assumed_utc_offset_seconds
                                            )
                                        )
                                    })
                                    .unwrap_or_default();
                                let destinations = items
                                    .iter()
                                    .flat_map(|item| &item.copies)
                                    .map(|c| {
                                        format!(
                                            "{}{} · {}",
                                            c.final_destination.display(),
                                            if c.required { "" } else { " (optional)" },
                                            plan_status_label(c.status)
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
                                                "{names} · {}{modified}",
                                                plan_status_label(status)
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
                    .hover(move |style| style.bg(p.selected))
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
        if self.history.is_empty() {
            panel = panel.child(div().p_5().text_color(p.muted)
                .child("No imports yet. Open a source, select media, then review an import to start your history."));
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
            let mut index = 0;
            while index < detail.imports.len() {
                let first = &detail.imports[index];
                let stem = std::path::Path::new(&first.destination_path).with_extension("");
                let mut group_end = index + 1;
                while group_end < detail.imports.len()
                    && std::path::Path::new(&detail.imports[group_end].destination_path)
                        .with_extension("")
                        == stem
                {
                    group_end += 1;
                }
                let group = &detail.imports[index..group_end];
                let destinations = group
                    .iter()
                    .map(|item| item.destination_path.clone())
                    .collect::<Vec<_>>()
                    .join(" + ");
                let modified = std::fs::metadata(&first.destination_path)
                    .and_then(|metadata| metadata.modified())
                    .ok()
                    .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                    .map(|duration| {
                        format_file_time(
                            duration.as_secs(),
                            self.preset.time_correction.assumed_utc_offset_seconds,
                        )
                    })
                    .unwrap_or_else(|| "unknown".into());
                panel = panel.child(
                    div()
                        .px_5()
                        .py_2()
                        .border_b_1()
                        .border_color(p.border)
                        .child(
                            div()
                                .text_sm()
                                .child(format!("{destinations} · {:?}", first.status)),
                        )
                        .child(div().text_xs().child(format!(
                            "Modified {modified} · Verification: {} · {}",
                            first.verification_method.as_deref().unwrap_or("pending"),
                            first.error.as_deref().unwrap_or("")
                        ))),
                );
                index = group_end;
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
        if self.incomplete_sessions.is_empty() && self.partial_files.is_empty() {
            panel = panel.child(div().mt_4().p_4().rounded_md().bg(p.card)
                .child("Nothing needs recovery. Interrupted imports and incomplete files will appear here."));
        }
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
                            .id(gpui::ElementId::named_usize("clean-partial", index))
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
            .child(settings_section("Source", p))
            .child(settings_field(
                "Current source alias (blank uses device name)",
                self.settings.source_alias.clone(),
                p,
                cx,
            ))
            .child(button(
                "Save source alias",
                cx.listener(|t, _, _, c| t.save_source_alias(c)),
            ))
            .child(settings_section("Preset and destinations", p))
            .child(div().text_xs().text_color(p.muted)
                .child("Choose a starting preset to replace the fields below."))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(chip(
                        "Everyday",
                        self.preset.name == "Everyday",
                        cx.listener(|t, _, _, c| t.choose_preset(false, c)),
                    ))
                    .child(chip(
                        "Organized",
                        self.preset.name == "Organized",
                        cx.listener(|t, _, _, c| t.choose_preset(true, c)),
                    )),
            )
            .child(settings_field(
                "Photo destination",
                self.settings.photo_root.clone(),
                p,
                cx,
            ))
            .child(settings_field(
                "Photo folder template",
                self.settings.photo_folder.clone(),
                p,
                cx,
            ))
            .child(settings_field(
                "Video destination",
                self.settings.video_root.clone(),
                p,
                cx,
            ))
            .child(settings_field(
                "Video folder template",
                self.settings.video_folder.clone(),
                p,
                cx,
            ))
            .child(settings_section("File naming", p))
            .child(settings_field(
                "Filename template",
                self.settings.filename.clone(),
                p,
                cx,
            ))
            .child(settings_section("Import rules", p))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        div()
                            .id("verification")
                            .cursor_pointer()
                            .p_2()
                            .bg(p.card).rounded_md().border_1().border_color(p.border)
                            .hover(move |style| style.bg(p.selected))
                            .child(format!("Verification: {:?}", self.preset.verification))
                            .on_click(cx.listener(|t, _, _, c| t.cycle_verification(c))),
                    )
                    .child(
                        div()
                            .id("bundle-policy")
                            .cursor_pointer()
                            .p_2()
                            .bg(p.card).rounded_md().border_1().border_color(p.border)
                            .hover(move |style| style.bg(p.selected))
                            .child(format!("Bundles: {:?}", self.preset.bundle_policy))
                            .on_click(cx.listener(|t, _, _, c| t.cycle_bundle_policy(c))),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        div()
                            .id("grouping")
                            .cursor_pointer()
                            .p_2()
                            .bg(p.card).rounded_md().border_1().border_color(p.border)
                            .hover(move |style| style.bg(p.selected))
                            .child(format!("Grouping: {:?}", self.preset.grouping))
                            .on_click(cx.listener(|t, _, _, c| t.cycle_grouping(c))),
                    )
                    .child(
                        div()
                            .id("collision")
                            .cursor_pointer()
                            .p_2()
                            .bg(p.card).rounded_md().border_1().border_color(p.border)
                            .hover(move |style| style.bg(p.selected))
                            .child(format!("Collision: {:?}", self.preset.collision))
                            .on_click(cx.listener(|t, _, _, c| t.cycle_collision(c))),
                    ),
            )
            .child(settings_field(
                "Additional photo extensions (comma separated)",
                self.settings.include_photo.clone(),
                p,
                cx,
            ))
            .child(settings_field(
                "Additional video extensions (comma separated)",
                self.settings.include_video.clone(),
                p,
                cx,
            ))
            .child(settings_field(
                "Excluded extensions (comma separated)",
                self.settings.exclude_extensions.clone(),
                p,
                cx,
            ))
            .child(settings_field(
                "Ignored media types (raw, photo, video, sidecar, unknown)",
                self.settings.ignore_types.clone(),
                p,
                cx,
            ))
            .child(settings_section("Capture time", p))
            .child(settings_field(
                "Clock correction (seconds)",
                self.settings.clock_seconds.clone(),
                p,
                cx,
            ))
            .child(settings_field(
                "Time-gap session threshold (minutes)",
                self.settings.gap_minutes.clone(),
                p,
                cx,
            ))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .text_color(p.muted)
                            .child("Timezone applied to capture times"),
                    )
                    .child(button(
                        format!(
                            "Timezone: {}",
                            timezone_label(
                                self.settings
                                    .timezone_seconds
                                    .read(cx)
                                    .value()
                                    .trim()
                                    .parse::<i32>()
                                    .ok()
                            )
                        ),
                        cx.listener(|t, _, _, c| {
                            t.timezone_menu_open = !t.timezone_menu_open;
                            c.notify()
                        }),
                    ))
                    .child(if self.timezone_menu_open {
                        div()
                            .id("timezone-menu")
                            .max_h(px(200.))
                            .overflow_y_scroll()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(timezone_choice(
                                "Capture metadata".into(),
                                self.settings.timezone_seconds.read(cx).value().trim().is_empty(),
                                u64::MAX,
                                p,
                                cx.listener(|t, _, _, c| t.set_timezone_offset(None, c)),
                            ))
                            .children(TIMEZONE_CHOICES.iter().map(|(label, seconds)| {
                                timezone_choice(
                                    (*label).to_string(),
                                    self.settings
                                        .timezone_seconds
                                        .read(cx)
                                        .value()
                                        .trim()
                                        .parse::<i32>()
                                        .ok()
                                        == Some(*seconds),
                                    (*seconds + 50_400) as u64,
                                    p,
                                    cx.listener(move |t, _, _, c| {
                                        t.set_timezone_offset(Some(*seconds), c)
                                    }),
                                )
                            }))
                            .into_any_element()
                    } else {
                        div().into_any_element()
                    })
                    .child(settings_field(
                        "Custom offset (seconds east of UTC; blank keeps metadata timezone)",
                        self.settings.timezone_seconds.clone(),
                        p,
                        cx,
                    )),
            )
            .child(settings_field(
                "Time-gap session threshold (minutes)",
                self.settings.gap_minutes.clone(),
                p,
                cx,
            ))
            .child(settings_section("Backup copies", p))
            .child(settings_field(
                "Backup photo root (blank disables backup)",
                self.settings.backup_photo_root.clone(),
                p,
                cx,
            ))
            .child(settings_field(
                "Backup video root (blank disables backup)",
                self.settings.backup_video_root.clone(),
                p,
                cx,
            ))
            .child(
                div()
                    .id("backup-required")
                    .cursor_pointer()
                    .p_2()
                    .bg(p.card).rounded_md().border_1().border_color(p.border)
                    .hover(move |style| style.bg(p.selected))
                    .child(format!("Backup required: {}", self.backup_required_choice))
                    .on_click(cx.listener(|t, _, _, c| t.toggle_backup_required(c))),
            )
            .child(primary_button(
                "Apply settings",
                p,
                self.ui.dark_mode,
                cx.listener(|t, _, _, c| t.apply_settings(c)),
            ))
            .into_any_element()
    }
}
fn timezone_choice(
    label: String,
    active: bool,
    id: u64,
    palette: Palette,
    handler: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(("timezone-choice", id))
        .cursor_pointer()
        .px_3()
        .py_2()
        .rounded_md()
        .text_sm()
        .bg(if active { palette.selected } else { palette.card })
        .hover(move |style| style.bg(palette.selected))
        .on_click(handler)
        .child(label)
}

fn settings_section(label: &'static str, palette: Palette) -> impl IntoElement {
    div().mt_5().pb_1().border_b_1().border_color(palette.border)
        .text_sm().font_weight(gpui::FontWeight::SEMIBOLD).child(label)
}
fn settings_field(
    label: &'static str,
    input: gpui::Entity<text_input::TextInput>,
    palette: Palette,
    cx: &Context<Browser>,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(div().text_xs().text_color(palette.muted).child(label))
        .child(themed_input(input, palette, cx))
        .into_any_element()
}

fn themed_input(
    input: gpui::Entity<text_input::TextInput>,
    palette: Palette,
    cx: &Context<Browser>,
) -> AnyElement {
    let focus = input.read(cx).focus_handle(cx);
    div()
        .w_full()
        .min_w_0()
        .track_focus(&focus)
        .border_1()
        .border_color(palette.border)
        .focus(move |style| style.border_color(palette.accent))
        .rounded_md()
        .overflow_hidden()
        .bg(palette.card)
        .text_color(palette.text)
        .child(input)
        .into_any_element()
}

fn sidebar_nav(
    label: &'static str,
    active: bool,
    palette: Palette,
    handler: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(label)
        .min_h(px(36.))
        .px_3()
        .py_2()
        .rounded_md()
        .cursor_pointer()
        .text_sm()
        .font_weight(if active { gpui::FontWeight::SEMIBOLD } else { gpui::FontWeight::NORMAL })
        .bg(if active { palette.selected } else { palette.panel })
        .hover(move |style| style.bg(palette.selected))
        .on_click(handler)
        .child(label)
}

fn primary_button(
    label: &'static str,
    palette: Palette,
    dark: bool,
    handler: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(label)
        .min_h(px(38.))
        .px_4()
        .py_2()
        .rounded_md()
        .cursor_pointer()
        .text_sm()
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .bg(if dark { palette.accent } else { rgb(0x247c66) })
        .text_color(if dark { palette.canvas } else { rgb(0xffffff) })
        .hover(move |style| style.bg(if dark { rgb(0xa1e8bf) } else { rgb(0x195d4c) }))
        .on_click(handler)
        .child(label)
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
            } else if self.state.source.is_none() {
                "Choose a source to begin".into()
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
                    .h(px(70.))
                    .px_6()
                    .flex()
                    .items_center()
                    .justify_between()
                    .bg(rgb(0x193c34))
                    .text_color(rgb(0xffffff))
                    .child(
                        div()
                            .flex().flex_col()
                            .child(div().text_xl().font_weight(gpui::FontWeight::BOLD).child("CapturePort"))
                            .child(div().text_xs().text_color(rgb(0xb9dfc9)).child("Photo and video ingest")),
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
                    .child(div().min_w_0().truncate().text_color(p.muted).child(status))
                    .child(div().flex().items_center().gap_3()
                        .child(format!(
                            "{} items · {} selected · {}",
                            self.state.len(),
                            summary.count,
                            format_size(summary.bytes)
                        ))
                        .child(if self.page == Page::Browser && summary.count > 0 && !self.importing && !self.planning && self.last_import_result.is_none() {
                            primary_button("Preview import", p, self.ui.dark_mode,
                                cx.listener(|t, _, w, c| {
                                    if t.plan.is_some() {
                                        t.page = Page::Preview;
                                        c.notify();
                                    } else {
                                        t.import_selected(&ImportSelected, w, c);
                                    }
                                }))
                                .into_any_element()
                        } else { div().into_any_element() })),
            )
    }
}

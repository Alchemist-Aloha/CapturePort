use crate::*;

impl Browser {
    /// Label for the open source: a user alias first, then a detected removable
    /// mount as its model over its mount dir, then the source's own display name.
    pub(crate) fn source_label(&self) -> Option<SourceLabel> {
        if let Some(alias) = &self.source_alias {
            return Some(SourceLabel {
                primary: alias.clone(),
                secondary: None,
            });
        }
        if let Some(root) = &self.filesystem_root
            && captureport_gphoto::discovery::likely_removable_mount(root)
        {
            return Some(removable_source_label(
                self.mounted_models.get(root).map(String::as_str),
                root,
            ));
        }
        self.state
            .source
            .as_ref()
            .and_then(|source| source.display_name.clone())
            .map(|name| SourceLabel {
                primary: name,
                secondary: None,
            })
    }
    pub(crate) fn sidebar(&mut self, sidebar_width: f32, cx: &mut Context<Self>) -> AnyElement {
        let p = Palette::new(self.ui.scheme, self.ui.dark_mode);
        let mut panel = div()
            .id("sidebar")
            .w_full()
            .flex_shrink_0()
            .h_full()
            .p(px(spacing::CONTENT))
            .border_r_1()
            .border_color(p.border)
            .bg(p.panel)
            .flex()
            .flex_col()
            .gap(px(spacing::CONTROL_GAP))
            .overflow_y_scroll()
            .track_scroll(&self.scrollbars.sidebar.handle)
            .scrollbar_width(px(spacing::SCROLLBAR_GUTTER))
            .child(
                div()
                    .text_xs()
                    .text_color(p.muted)
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .flex_shrink_0()
                    .child("Sources"),
            )
            .child(button(
                Icon::Folder,
                if sidebar_width < 232. {
                    "Open folder"
                } else {
                    "Open folder…"
                },
                p,
                cx.listener(|t, _, w, c| t.open_folder(&OpenFolder, w, c)),
            ))
            .child(button(
                Icon::Refresh,
                match (self.discovering, sidebar_width < 232.) {
                    (true, true) => "Scanning…",
                    (true, false) => "Scanning devices…",
                    (false, true) => "Refresh",
                    (false, false) => "Refresh devices",
                },
                p,
                cx.listener(|t, _, w, c| t.discover_sources(&DiscoverSources, w, c)),
            ));
        for (index, card) in self.unmounted_cards.iter().enumerate() {
            panel = panel.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(spacing::TIGHT))
                    .child(
                        div()
                            .text_sm()
                            .child(format!("Card · {} · Not mounted", card.label)),
                    )
                    .child(button(
                        Icon::Storage,
                        if self.mounting.as_ref() == Some(&card.path) {
                            "Mounting…"
                        } else {
                            "Mount"
                        },
                        p,
                        cx.listener(move |t, _, _, c| t.mount_card(index, c)),
                    )),
            );
        }
        for (index, discovered) in self.discovered_sources.iter().enumerate() {
            let label = match discovered {
                captureport_gphoto::discovery::DiscoveredSource::Ptp(camera) => SourceLabel {
                    primary: format!("Camera · {}", camera.model),
                    secondary: None,
                },
                captureport_gphoto::discovery::DiscoveredSource::MountedFilesystem {
                    path, ..
                } => {
                    removable_source_label(self.mounted_models.get(path).map(String::as_str), path)
                }
            };
            let label = self
                .discovery_aliases
                .get(discovered.stable_id())
                .cloned()
                .map(|alias| SourceLabel {
                    primary: alias,
                    secondary: None,
                })
                .unwrap_or(label);
            panel = panel.child(
                div()
                    .id(gpui::ElementId::named_usize("source", index))
                    .cursor_pointer()
                    .px(px(spacing::CONTENT))
                    .py(px(spacing::TIGHT))
                    .flex()
                    .items_center()
                    .text_sm()
                    .min_h(px(spacing::CONTROL_HEIGHT))
                    .flex_shrink_0()
                    .rounded_sm()
                    .border_1()
                    .border_color(p.border)
                    .bg(
                        if self.state.source.as_ref().is_some_and(|source| {
                            source.stable_id.as_deref() == Some(discovered.stable_id())
                        }) {
                            p.selected
                        } else {
                            p.card
                        },
                    )
                    .hover(move |style| style.bg(p.selected))
                    .on_click(cx.listener(move |t, _, _, c| t.open_discovered(index, c)))
                    .gap(px(spacing::CONTROL_GAP))
                    .child(icon(match discovered {
                        captureport_gphoto::discovery::DiscoveredSource::Ptp(_) => Icon::Camera,
                        captureport_gphoto::discovery::DiscoveredSource::MountedFilesystem { .. } => Icon::Storage,
                    }, p.text))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(div().w_full().truncate().child(label.primary))
                            .children(label.secondary.map(|secondary| {
                                div()
                                    .w_full()
                                    .truncate()
                                    .text_xs()
                                    .text_color(p.muted)
                                    .child(secondary)
                            })),
                    ),
            );
        }
        panel = panel
            .children(
                self.source_label()
                    .filter(|_| self.page != Page::Browser)
                    .map(|label| {
                        div()
                            .flex_shrink_0()
                            .text_xs()
                            .text_color(p.muted)
                            .truncate()
                            .child(format!("Current: {}", label.joined()))
                    }),
            )
            .child(
                div()
                    .mt(px(spacing::SECTION - spacing::CONTROL_GAP))
                    .text_xs()
                    .text_color(p.muted)
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("Workspace"),
            )
            .child(sidebar_nav(
                Icon::Photo,
                "Browse media",
                self.page == Page::Browser,
                p,
                cx.listener(|t, _, _, c| {
                    t.page = Page::Browser;
                    c.notify()
                }),
            ))
            .child(if self.plan.is_some() {
                sidebar_nav(
                    Icon::Review,
                    "Review import",
                    self.page == Page::Preview,
                    p,
                    cx.listener(|t, _, _, c| {
                        t.page = Page::Preview;
                        c.notify()
                    }),
                )
                .into_any_element()
            } else {
                div().hidden().into_any_element()
            })
            .child(sidebar_nav(
                Icon::History,
                "History",
                self.page == Page::History,
                p,
                cx.listener(|t, _, w, c| t.history(&ShowHistory, w, c)),
            ))
            .child(sidebar_nav(
                Icon::Revert,
                "Recovery",
                self.page == Page::Recovery,
                p,
                cx.listener(|t, _, _, c| t.show_recovery(c)),
            ))
            .child(sidebar_nav(
                Icon::Settings,
                "Settings",
                self.page == Page::Settings,
                p,
                cx.listener(|t, _, _, c| t.show_settings(c)),
            ))
            .child(if self.reconcile_cancellation.is_some() {
                button(
                    Icon::Cancel,
                    "Cancel library scan",
                    p,
                    cx.listener(|t, _, w, c| t.cancel_reconcile(&CancelReconcile, w, c)),
                )
                .into_any_element()
            } else {
                div().hidden().into_any_element()
            })
            .when(cfg!(debug_assertions), |view| {
                view.child(button(
                    Icon::Play,
                    "10k-item demo",
                    p,
                    cx.listener(|t, _, w, c| t.open_demo(&OpenDemo, w, c)),
                ))
            })
            .child(if self.importing {
                button(
                    Icon::Cancel,
                    "Cancel import",
                    p,
                    cx.listener(|t, _, w, c| t.cancel_import(&CancelImport, w, c)),
                )
                .into_any_element()
            } else {
                div().hidden().into_any_element()
            })
            .child(
                div()
                    .mt(px(spacing::SECTION - spacing::CONTROL_GAP))
                    .flex_shrink_0()
                    .text_xs()
                    .text_color(p.muted)
                    .child(format!("Preset: {}", self.preset.name)),
            );
        div()
            .w(px(sidebar_width))
            .flex_shrink_0()
            .h_full()
            .relative()
            .child(panel)
            .child(self.scrollbars.sidebar.element(p.border, p.muted, p.accent))
            .into_any_element()
    }
    pub(crate) fn browser_panel(
        &mut self,
        window: &mut Window,
        sidebar_width: f32,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let p = Palette::new(self.ui.scheme, self.ui.dark_mode);
        let visible = self.visible_ids.len();
        let candidates = visible_capture_ids(
            self.state.visible_items().into_iter().map(|item| item.id),
            &self.bundle_owner,
        );
        let waiting = candidates.len().saturating_sub(visible);
        let failed = candidates
            .iter()
            .filter(|id| {
                let preview = self.preview_id(**id);
                self.failed_thumbnails.contains(&preview)
                    || self.state.item(preview).is_some_and(|item| {
                        matches!(item.metadata, captureport_core::MetadataState::Failed(_))
                    })
            })
            .count();
        let hidden_selected = self
            .state
            .items()
            .filter(|item| {
                self.state.is_selected(item.id)
                    && (!self.thumbnail_paths.contains_key(&self.preview_id(item.id))
                        || !matches!(item.metadata, captureport_core::MetadataState::Ready(_)))
            })
            .count();
        let mut readiness = vec![format!("{visible} ready")];
        if waiting > failed {
            readiness.push(format!("{} loading", waiting - failed));
        }
        if failed > 0 {
            readiness.push(format!("{failed} unavailable"));
        }
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
            self.gallery_groups
                .iter()
                .flat_map(|group| {
                    group
                        .ids
                        .chunks(columns)
                        .enumerate()
                        .map(|(index, ids)| GalleryRow {
                            header: (index == 0).then(|| (group.key.clone(), group.title.clone())),
                            ids: ids.to_vec(),
                        })
                        .collect::<Vec<_>>()
                })
                .collect()
        } else {
            self.visible_ids
                .chunks(columns)
                .map(|ids| GalleryRow {
                    header: None,
                    ids: ids.to_vec(),
                })
                .collect()
        };
        let row_count = rows.len();
        let rows = std::sync::Arc::new(rows);
        let source_label = self.source_label().unwrap_or(SourceLabel {
            primary: "No source open".into(),
            secondary: None,
        });
        div().flex_1().min_w_0().min_h_0().flex().flex_col()
            .when(self.state.source.is_some(), |view| view.child(div().px(px(spacing::CONTENT)).pt(px(spacing::CONTENT)).pb(px(spacing::CONTROL_GAP)).flex().flex_col().gap(px(spacing::CONTENT)).border_b_1().border_color(p.border)
                .child(div().min_w_0().flex().items_baseline().justify_between().gap(px(spacing::CONTENT))
                    .child(div().flex_1().min_w_0().flex().flex_col()
                        .child(div().w_full().font_family(DISPLAY_FONT).text_lg().font_weight(gpui::FontWeight::SEMIBOLD).truncate().child(source_label.primary))
                        .children(source_label.secondary.map(|secondary| div().w_full().text_xs().text_color(p.muted).truncate().child(secondary))))
                    .child(div().flex_shrink_0().text_xs().text_color(p.muted).child(readiness.join(" · "))))
                .children((hidden_selected > 0).then(|| div().text_xs().text_color(p.muted)
                    .child(format!("{hidden_selected} selected files have no preview. Review their import paths."))))
                .when(!self.state.is_empty(), |view| view.child(div().flex().flex_wrap().gap(px(spacing::TIGHT))
                    .child(chip("All",self.state.filter==MediaFilter::All, p, cx.listener(|t,_,_,c|t.filter(MediaFilter::All,c))))
                    .child(chip("Photos",self.state.filter==MediaFilter::Photos, p, cx.listener(|t,_,_,c|t.filter(MediaFilter::Photos,c))))
                    .child(chip("Videos",self.state.filter==MediaFilter::Videos, p, cx.listener(|t,_,_,c|t.filter(MediaFilter::Videos,c))))
                    .child(chip("New",self.state.filter==MediaFilter::New, p, cx.listener(|t,_,_,c|t.filter(MediaFilter::New,c))))
                    .child(chip("Imported",self.state.filter==MediaFilter::Imported, p, cx.listener(|t,_,_,c|t.filter(MediaFilter::Imported,c))))
                    .child(chip("Possible",self.state.filter==MediaFilter::PossibleDuplicates, p, cx.listener(|t,_,_,c|t.filter(MediaFilter::PossibleDuplicates,c))))))))
            .when(!self.state.is_empty(), |view| view.child(div().px(px(spacing::CONTENT)).py(px(spacing::CONTROL_GAP)).flex().flex_wrap().items_center().gap(px(spacing::CONTROL_GAP)).border_b_1().border_color(p.border)
                    .child(button(Icon::SelectAll, "Select all", p, cx.listener(|t,_,w,c|t.select_all(&SelectAll,w,c))))
                    .child(button(Icon::SelectNew, "Select new", p, cx.listener(|t,_,w,c|t.select_new(&SelectAllNew,w,c))))
                    .child(button(Icon::Clear, "Clear", p, cx.listener(|t,_,w,c|t.select_none(&SelectNone,w,c))))
                    .child(div().id("mark-selected-imported").min_h(px(spacing::CONTROL_HEIGHT)).flex().items_center()
                        .gap(px(spacing::CONTROL_GAP))
                        .px(px(spacing::CONTENT)).py(px(spacing::TIGHT)).rounded_sm().text_sm().bg(p.card)
                        .text_color(if self.can_mark_selected_imported() { p.text } else { p.muted })
                        .when(self.can_mark_selected_imported(), |button| button.cursor_pointer()
                            .hover(move |style| style.bg(p.selected))
                            .on_click(cx.listener(|t,_,w,c|t.mark_selected_imported(&MarkSelectedImported,w,c))))
                        .child(icon(Icon::Confirm, if self.can_mark_selected_imported() { p.text } else { p.muted }))
                        .child(if self.marking_imported { "Marking…" } else { "Mark as imported" }))
                .child(button(Icon::ViewOptions, if self.show_view_options { "Hide view options" } else { "View options" }, p,
                    cx.listener(|t,_,_,c| { t.show_view_options = !t.show_view_options; c.notify() })))))
            .child(if self.show_view_options && !self.state.is_empty() {
                div().px(px(spacing::CONTENT)).py(px(spacing::CONTENT)).flex().flex_wrap().items_center().gap(px(spacing::CONTENT)).border_b_1().border_color(p.border).bg(p.panel)
                    .child(div().flex().items_center().gap(px(spacing::CONTROL_GAP))
                        .child(div().text_sm().text_color(p.muted).child("Sort by"))
                        .child(chip("Time",self.state.sort==MediaSort::CaptureTime, p, cx.listener(|t,_,_,c|t.sort(MediaSort::CaptureTime,c))))
                        .child(chip("Name",self.state.sort==MediaSort::Name, p, cx.listener(|t,_,_,c|t.sort(MediaSort::Name,c)))))
                    .child(div().flex().items_center().gap(px(spacing::CONTROL_GAP))
                        .child(div().text_sm().text_color(p.muted).child("Thumbnail size"))
                        .child(div().id("thumbnail-size-slider").flex().items_center().gap(px(spacing::TIGHT))
                            .children((0..=4).map(|step| {
                                let active = step <= self.ui.thumbnail_size;
                                div().id(gpui::ElementId::named_usize("thumbnail-size-step", step as usize)).w(px(24.)).h(px(28.))
                                    .flex().items_center().cursor_pointer()
                                    .on_click(cx.listener(move |t,_,_,c| t.set_thumbnail_size(step,c)))
                                    .on_mouse_move(cx.listener(move |t,e: &gpui::MouseMoveEvent,_,c| {
                                        if e.dragging() { t.set_thumbnail_size(step,c); }
                                    }))
                                    .child(div().w_full().h(px(if active { 6. } else { 4. }))
                                        .rounded_full().bg(if active { p.accent } else { p.border }))
                            })))
                        .child(div().text_xs().text_color(p.muted).child(format!("{} px", image_height.round() as u32))))
                    .child(div().flex().items_center().gap(px(spacing::CONTROL_GAP))
                        .child(div().text_sm().text_color(p.muted).child("Session gap"))
                        .child(div().id("gallery-gap-slider").flex().items_center().gap(px(spacing::TIGHT))
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
            } else { div().hidden().into_any_element() })
            .child(self.expanded_bundle_panel(cx))
            .child(if visible==0 {
                div().flex_1().flex().items_center().justify_center().px(px(spacing::CONTENT))
                    .child(div().flex().flex_col().items_center().gap(px(spacing::CONTENT))
                        .child(div().font_family(DISPLAY_FONT).text_lg().font_weight(gpui::FontWeight::SEMIBOLD)
                            .child(if self.scanning { "Scanning source…" } else if waiting > failed { "Preparing previews…" } else if failed > 0 { "Previews unavailable" } else if self.state.source.is_none() { "Start with a source" } else if self.state.filter == MediaFilter::All { "No media found in this source" } else { "No media matches this filter" }))
                        .child(div().text_sm().text_color(p.muted)
                            .child(if self.scanning { "Media appears after metadata and its thumbnail are ready." } else if failed > 0 && waiting == failed { "Some files could not be previewed. Check the source or video decoder, then reopen it to retry." } else if waiting > 0 { "Metadata and thumbnails are loading." } else if self.state.source.is_none() { if self.discovered_sources.is_empty() && !self.unmounted_cards.is_empty() { "Mount the card in Sources to browse media." } else if self.discovered_sources.is_empty() { "Open a folder or connect a camera to browse media." } else { "Choose a discovered camera or card in Sources to browse media." } } else if self.state.filter == MediaFilter::All { "Try another source or check that it contains supported media." } else { "Choose All to see every capture in this source." }))
                        .child(if self.state.source.is_none() && !self.scanning {
                            div().flex().flex_wrap().justify_center().gap(px(spacing::CONTROL_GAP))
                                .child(primary_button(Icon::Folder, "Open a folder…", p,
                                    cx.listener(|t, _, w, c| t.open_folder(&OpenFolder, w, c))))
                                .into_any_element()
                        } else if self.state.filter != MediaFilter::All && !self.scanning {
                            button(Icon::Photo, "Show all media", p,  cx.listener(|t, _, _, c| t.filter(MediaFilter::All, c))).into_any_element()
                        } else { div().hidden().into_any_element() }))
                    .into_any_element()
            } else {
                div().relative().flex_1().min_h_0().flex().flex_col().child(
                uniform_list(("media-grid", columns * 5 + self.ui.thumbnail_size as usize + if grouped { 100 } else { 0 }),row_count,cx.processor(move |t,range:std::ops::Range<usize>,_,cx|range.map(|row| {
                    let GalleryRow { header, ids } = &rows[row];
                    let mut cards=div().w_full().flex().gap(px(spacing::CONTENT));
                    for column in 0..columns {
                        if let Some(&id)=ids.get(column) {
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
                                let size_label=format_size(total_size);
                                let time_label=t.thumbnail_modified.get(&id).copied().map(|seconds| format_file_time_compact(seconds,timezone)).unwrap_or_default();
                                let picture = {
                                    let preview_id = t.preview_id(id);
                                    let base = if let Some(path)=t.thumbnail_paths.get(&preview_id) {
                                        div().w_full().h(px(image_height)).flex().items_center().justify_center().bg(p.placeholder).overflow_hidden()
                                            .child(img(path.clone()).h(px(image_height)).max_w_full().object_fit(ObjectFit::Contain))
                                    } else {
                                        div().w_full().h(px(image_height)).bg(p.placeholder)
                                    };
                                    div().id(("media-open", id.0)).relative().w_full().cursor_pointer()
                                        .on_click(cx.listener(move|t,_,_,c|t.toggle_bundle(id,c)))
                                        .child(base)
                                        .child(div().absolute().top(px(spacing::TIGHT)).right(px(spacing::TIGHT)).px(px(spacing::TIGHT)).py(px(1.)).rounded_sm()
                                            .bg(image_badge_surface().0).text_color(image_badge_surface().1)
                                            .flex().items_center().gap(px(spacing::TIGHT))
                                            .text_xs().font_weight(gpui::FontWeight::SEMIBOLD)
                                            .child(icons::icon_sized(badge.0, icons::ICON_SIZE_COMPACT, image_badge_surface().1))
                                            .child(badge.1))
                                        .child(if partly_selected {
                                            div().absolute().top(px(spacing::TIGHT)).left(px(spacing::TIGHT)).px(px(spacing::CONTROL_GAP)).py(px(spacing::TIGHT)).rounded_sm()
                                                .bg(image_badge_surface().0).text_color(image_badge_surface().1).text_xs()
                                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                                .child(if selected { "Selected" } else { "Part selected" })
                                                .into_any_element()
                                        } else { div().hidden().into_any_element() })
                                        .into_any_element()
                                };
                                // Block flow keeps truncated text out of flex's intrinsic-width
                                // measurement, where GPUI can cache an ellipsis-only filename.
                                // The caption carries its own surface: an unselected tile's
                                // canvas background is the page background, so the title and
                                // metadata would otherwise float on the grid with no tile edge.
                                let metadata = div().w_full().h(px(spacing::MEDIA_DETAILS_HEIGHT)).px(px(spacing::CONTROL_GAP)).py(px(spacing::CONTROL_GAP)).min_w_0()
                                    .bg(if selected { p.selected } else { p.card })
                                    .child(div().flex().items_center().gap(px(spacing::TIGHT))
                                        .child(div().id(("media-name", id.0)).flex_1().min_w_0().cursor_pointer().rounded_sm()
                                            .on_click(cx.listener(move |t, _, _, c| {
                                                c.stop_propagation();
                                                t.toggle_group_selection(id, c);
                                            }))
                                            .hover(move |style| style.bg(p.selected))
                                            .child(div().w_full().text_sm().font_weight(gpui::FontWeight::SEMIBOLD)
                                                .truncate().child(item.source_name.clone())))
                                        .child(t.media_detail_trigger(id, p, cx)))
                                    .child(div().mt(px(spacing::TIGHT)).text_xs().text_color(p.muted).truncate().child(
                                        if time_label.is_empty() { size_label } else { format!("{time_label} · {size_label}") }))
                                    .child(div().id(("media-status", id.0)).mt(px(spacing::TIGHT)).truncate().text_xs().font_weight(gpui::FontWeight::SEMIBOLD).cursor_pointer()
                                        .on_click(cx.listener(move|t,_,_,c|t.toggle_bundle(id,c)))
                                        .text_color(if partly_selected { p.text } else { p.muted })
                                        .child(import_status_line(item)))
                                    .child(div().flex().items_center().justify_between().gap(px(spacing::TIGHT))
                                        .child(div().id(("media-disclosure", id.0)).when(members.is_none(), |style| style.hidden()).text_xs().text_color(p.muted).cursor_pointer()
                                            .on_click(cx.listener(move |t,_,_,c| t.toggle_bundle(id,c)))
                                            .child(if members.is_some() {
                                                if expanded { "Hide files" } else { "View files" }
                                            } else { "" }))
                                        );
                                cards=cards.child(div().id(("media",id.0)).flex_1().min_w_0()
                                    .overflow_hidden().rounded_sm().border_1()
                                    .border_color(if partly_selected || expanded {p.accent}else{p.canvas})
                                    .bg(if selected{p.selected}else{p.canvas})
                                    .child(picture).child(metadata));
                            }
                        } else { cards=cards.child(div().flex_1()); }
                    }
                    // UniformList does not scope row children: repeated Rename/Save
                    // buttons must live under a distinct row ID to receive clicks.
                    let mut view=div().id(("media-row", ids[0].0)).w_full().h(px(image_height + spacing::MEDIA_ROW_CHROME + if grouped { spacing::GALLERY_ROW_CHROME } else { 0. })).flex().flex_col().px(px(spacing::CONTENT)).py(px(spacing::CONTROL_GAP));
                    if grouped {
                        let heading = if let Some((key, title)) = header {
                            if t.gallery_edit_key.as_ref() == Some(key) {
                                div().w_full().h(px(spacing::CONTROL_HEIGHT)).flex().items_center().gap(px(spacing::CONTROL_GAP))
                                    .child(div().flex_1().min_w_0()
                                        .child(themed_input(t.gallery_edit_input.clone(), p, cx)))
                                    .child(button(Icon::Confirm, "Save", p,  cx.listener(|t,_,_,c| t.save_gallery_name(c))))
                            } else {
                                let key = key.clone();
                                let name_key = key.clone();
                                div().w_full().h(px(spacing::CONTROL_HEIGHT)).flex().items_center().gap(px(spacing::CONTROL_GAP))
                                    .child(
                                        div()
                                            .id(gpui::ElementId::Name(format!("gallery-name-{key}").into()))
                                            .flex_1().min_w_0().font_family(DISPLAY_FONT).text_base().font_weight(gpui::FontWeight::SEMIBOLD).truncate()
                                            .cursor_pointer().rounded_sm()
                                            .hover(move |style| style.bg(p.selected))
                                            .on_click(cx.listener(move |t, _, _, c| {
                                                t.toggle_gallery_selection(&name_key, c)
                                            }))
                                            .child(title.clone()),
                                    )
                                    .child(button(Icon::Rename, "Rename", p,  cx.listener(move |t,_,w,c| t.edit_gallery(key.clone(),w,c))))
                            }
                        } else { div().h(px(spacing::CONTROL_HEIGHT)) };
                        view=view.child(heading.mb(px(spacing::CONTROL_GAP)));
                    }
                    view.child(cards)
                }).collect())).track_scroll(self.scrollbars.media.list.clone()).pr(px(spacing::SCROLLBAR_GUTTER)).min_h_0().flex_1())
                    .child(self.scrollbars.media.element(p.border, p.muted, p.accent))
                    .into_any_element()
            }).into_any_element()
    }
    pub(crate) fn expanded_bundle_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(primary) = self.expanded_bundle else {
            return div().into_any_element();
        };
        let Some(members) = self.bundles.get(&primary) else {
            return div().into_any_element();
        };
        let p = Palette::new(self.ui.scheme, self.ui.dark_mode);
        let (_, label) = bundle_type_label(
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
            .track_scroll(&self.scrollbars.bundle.handle)
            .scrollbar_width(px(spacing::SCROLLBAR_GUTTER))
            .px(px(spacing::CONTENT))
            .py(px(spacing::CONTENT))
            .border_b_1()
            .border_color(p.border)
            .bg(p.panel)
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap(px(spacing::CONTROL_GAP))
                    .child(
                        div()
                            .id(("group-name", primary.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .cursor_pointer()
                            .on_click(
                                cx.listener(move |t, _, _, c| t.toggle_group_selection(primary, c)),
                            )
                            .child(format!(
                                "{title} · {selected_count}/{} selected",
                                members.len()
                            )),
                    )
                    .child(button(
                        Icon::Collapse,
                        "Collapse group",
                        p,
                        cx.listener(move |t, _, _, c| t.toggle_bundle(primary, c)),
                    )),
            )
            .child(
                div()
                    .my(px(spacing::CONTROL_GAP))
                    .flex()
                    .flex_wrap()
                    .gap(px(spacing::CONTROL_GAP))
                    .child(button(
                        Icon::SelectAll,
                        "Select all in group",
                        p,
                        cx.listener(move |t, _, _, c| t.select_bundle_members(primary, true, c)),
                    ))
                    .child(button(
                        Icon::Clear,
                        "Deselect all in group",
                        p,
                        cx.listener(move |t, _, _, c| t.select_bundle_members(primary, false, c)),
                    )),
            );
        for &id in members {
            let Some(item) = self.state.item(id) else {
                continue;
            };
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
                    .ml(px(spacing::CONTENT))
                    .pl(px(spacing::CONTENT))
                    .py(px(spacing::CONTROL_GAP))
                    .border_l_1()
                    .border_color(p.border)
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap(px(spacing::CONTROL_GAP))
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
                                media_type_badge(item.media_type).1,
                                format_size(item.size),
                                import_status_line(item)
                            ))),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(spacing::CONTROL_GAP))
                            .child(self.media_detail_trigger(id, p, cx))
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child(if selected { "Selected" } else { "Select" }),
                            ),
                    ),
            );
        }
        div()
            .relative()
            .flex_shrink_0()
            .max_h(px(240.))
            .min_h_0()
            .flex()
            .flex_col()
            .child(panel)
            .child(self.scrollbars.bundle.element(p.border, p.muted, p.accent))
            .into_any_element()
    }
}

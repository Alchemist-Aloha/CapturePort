use crate::*;

impl Browser {
    pub(crate) fn preview_panel(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let p = Palette::new(self.ui.scheme, self.ui.dark_mode);
        let Some(plan) = &self.plan else {
            return div()
                .flex_1()
                .p(px(spacing::CONTENT))
                .child("No preview yet")
                .into_any_element();
        };
        let count = plan.items.len();
        let groups = group_plan_items(&plan.items);
        let blocked = plan
            .items
            .iter()
            .filter(|item| blocks_import(item.status))
            .count();
        let skipped = plan
            .items
            .iter()
            .filter(|item| item.status == PlanStatus::Skipped)
            .count();
        div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .child(div().px(px(spacing::CONTENT)).py(px(spacing::CONTENT)).border_b_1().border_color(p.border)
                .child(div().flex().flex_wrap().items_center().justify_between().gap(px(spacing::CONTROL_GAP))
                    .child(div().font_family(DISPLAY_FONT).text_lg().font_weight(gpui::FontWeight::SEMIBOLD)
                        .child(format!("Review import · {count} files")))
                    .child(button(Icon::Back, "Back to media", p,  cx.listener(|t, _, _, c| { t.page = Page::Browser; c.notify() }))))
                .child(div().text_sm().text_color(p.muted)
                    .child(if blocked > 0 { format!("{blocked} blocked · Review the affected paths below") } else { "Check each final destination before copying.".into() })))
            .child(div().px(px(spacing::CONTENT)).py(px(spacing::CONTENT)).flex().flex_wrap().items_center().justify_between().gap(px(spacing::CONTENT)).border_b_1().border_color(p.border).bg(p.panel)
                .child(div().text_sm().child(format!(
                    "{} to copy · {skipped} skipped · {blocked} blocked · {} preset · {:?} verification",
                    count.saturating_sub(blocked + skipped), plan.preset_name, plan.verification
                )))
                .child(if blocked == 0 && count > 0 && !self.importing && self.last_import_result.is_none() {
                    primary_button(Icon::Import, "Confirm import", p,
                        cx.listener(|t, _, w, c| t.import_selected(&ImportSelected, w, c)))
                        .into_any_element()
                } else if blocked > 0 {
                    div().flex().items_center().gap(px(spacing::CONTROL_GAP))
                        .child(status_chip("Import blocked", p))
                        .child(button(Icon::Settings, "Edit import settings", p,  cx.listener(|t, _, _, c| t.show_settings(c))))
                        .into_any_element()
                } else { div().hidden().into_any_element() }))
            .child(
                if self.last_import_result.is_some() && self.filesystem_root.is_some() {
                    if self.deletion_armed {
                        // State the scope before acting: this is the only
                        // irreversible action in the product.
                        div()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .justify_between()
                            .gap(px(spacing::CONTENT))
                            .p(px(spacing::CONTENT))
                            .rounded_sm()
                            .border_1()
                            .border_color(p.danger)
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .min_w_0()
                                    .gap(px(spacing::TIGHT))
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .child(self.deletion_scope().unwrap_or_else(|| {
                                                "Delete verified originals from this source".into()
                                            })),
                                    )
                                    .child(div().text_xs().text_color(p.muted).child(
                                        "This cannot be undone. Only files whose required copies verified are removed.",
                                    )),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(spacing::CONTROL_GAP))
                                    .child(button(
                                        Icon::Revert,
                                        "Keep originals",
                                        p,
                                        cx.listener(|t, _, _, c| t.cancel_delete_sources(c)),
                                    ))
                                    .child(danger_button(
                                        Icon::Delete,
                                        "Delete originals",
                                        p,
                                        cx.listener(|t, _, _, c| t.delete_sources(c)),
                                    )),
                            )
                            .into_any_element()
                    } else {
                        // Quiet by default: the loudest thing on this screen
                        // must not be the invitation to erase the originals.
                        button(
                            Icon::Delete,
                            "Delete verified originals…",
                            p,
                            cx.listener(|t, _, _, c| t.delete_sources(c)),
                        )
                        .into_any_element()
                    }
                } else {
                    div().into_any_element()
                },
            )
            .child(
                div().relative().flex_1().min_h_0().flex().flex_col().child(
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
                                    .px(px(spacing::CONTENT))
                                    .py(px(spacing::CONTROL_GAP))
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
                .track_scroll(self.scrollbars.preview.list.clone())
                .pr(px(spacing::SCROLLBAR_GUTTER))
                .min_h_0()
                .flex_1())
                    .child(self.scrollbars.preview.element(p.border, p.muted, p.accent)),
            )
            .into_any_element()
    }
    pub(crate) fn history_panel(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let p = Palette::new(self.ui.scheme, self.ui.dark_mode);
        let mut panel = div()
            .id("history-panel")
            .flex_1()
            .min_h_0()
            .min_w_0()
            .flex()
            .flex_col()
            .overflow_y_scroll()
            .track_scroll(&self.scrollbars.history.handle)
            .scrollbar_width(px(spacing::SCROLLBAR_GUTTER))
            .child(
                div()
                    .px(px(spacing::CONTENT))
                    .py(px(spacing::CONTENT))
                    .border_b_1()
                    .border_color(p.border)
                    .font_family(DISPLAY_FONT)
                    .text_lg()
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
                    .px(px(spacing::CONTENT))
                    .py(px(spacing::CONTENT))
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
            panel = panel.child(div().p(px(spacing::CONTENT)).text_color(p.muted)
                .child("No imports yet. Open a source, select media, then review an import to start your history."));
        }
        if let Some(detail) = &self.session_detail {
            panel = panel.child(
                div()
                    .px(px(spacing::CONTENT))
                    .py(px(spacing::CONTENT))
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
                        .px(px(spacing::CONTENT))
                        .py(px(spacing::CONTROL_GAP))
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
        div()
            .relative()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .flex()
            .flex_col()
            .child(panel)
            .child(self.scrollbars.history.element(p.border, p.muted, p.accent))
            .into_any_element()
    }
    pub(crate) fn recovery_panel(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let p = Palette::new(self.ui.scheme, self.ui.dark_mode);
        let mut panel = div()
            .id("recovery-panel")
            .flex_1()
            .min_h_0()
            .min_w_0()
            .p(px(spacing::CONTENT))
            .flex()
            .flex_col()
            .gap(px(spacing::CONTROL_GAP))
            .overflow_y_scroll()
            .track_scroll(&self.scrollbars.recovery.handle)
            .scrollbar_width(px(spacing::SCROLLBAR_GUTTER))
            .child(
                div()
                    .font_family(DISPLAY_FONT)
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
            panel = panel.child(div().mt(px(spacing::CONTENT)).p(px(spacing::CONTENT)).rounded_sm().bg(p.card)
                .child("Nothing needs recovery. Interrupted imports and incomplete files will appear here."));
        }
        for session in &self.incomplete_sessions {
            panel = panel.child(div().p(px(spacing::CONTENT)).bg(p.card).rounded_sm().child(
                format!(
                    "Session {} · started {} · {} recorded copies",
                    session.session.id,
                    session.session.started_at,
                    session.imports.len()
                ),
            ));
        }
        if !self.incomplete_sessions.is_empty() {
            panel = panel.child(button(
                Icon::AddToList,
                "Review remaining files",
                p,
                cx.listener(|t, _, _, c| t.resume_from_current_source(c)),
            ));
        }
        for (index, partial) in self.partial_files.iter().enumerate() {
            panel = panel.child(
                div()
                    .p(px(spacing::CONTENT))
                    .bg(p.card)
                    .rounded_sm()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(format!(
                        "{} · {}",
                        partial.path.display(),
                        format_size(partial.size)
                    ))
                    .child(if self.armed_partial == Some(index) {
                        div()
                            .flex()
                            .items_center()
                            .gap(px(spacing::CONTROL_GAP))
                            .child(button(
                                Icon::Revert,
                                "Keep file",
                                p,
                                cx.listener(|t, _, _, c| t.cancel_clean_partial(c)),
                            ))
                            .child(danger_button(
                                Icon::Delete,
                                "Confirm delete",
                                p,
                                cx.listener(move |t, _, _, c| t.clean_partial(index, c)),
                            ))
                            .into_any_element()
                    } else {
                        button(
                            Icon::Delete,
                            "Clean incomplete file",
                            p,
                            cx.listener(move |t, _, _, c| t.clean_partial(index, c)),
                        )
                        .into_any_element()
                    }),
            );
        }
        div()
            .relative()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .flex()
            .flex_col()
            .child(panel)
            .child(
                self.scrollbars
                    .recovery
                    .element(p.border, p.muted, p.accent),
            )
            .into_any_element()
    }
    fn saved_presets_controls(&self, cx: &mut Context<Self>) -> AnyElement {
        use saved_presets::PresetAction;
        let p = Palette::new(self.ui.scheme, self.ui.dark_mode);
        let selected = self
            .saved_presets
            .iter()
            .find(|preset| Some(preset.id) == self.selected_preset);
        let mut controls = div().flex().flex_col().gap(px(spacing::CONTROL_GAP))
            .child(div().text_xs().text_color(p.muted)
                .child("Saved presets include every setting below: import rules, browsing, appearance, and the source-alias field."));
        if self.saved_presets.is_empty() && !self.preset_busy {
            controls = controls.child(div().text_sm().text_color(p.muted).child(
                "No saved presets yet. Enter a name and create one from your current settings.",
            ));
        } else {
            controls = controls.child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(spacing::CONTROL_GAP))
                    .children(self.saved_presets.iter().map(|preset| {
                        let id = preset.id;
                        div().id(("saved-preset", id as u64)).child(chip(
                            preset.name.clone(),
                            self.selected_preset == Some(id),
                            p,
                            cx.listener(move |t, _, _, c| t.load_saved_preset(id, c)),
                        ))
                    })),
            );
        }
        controls = controls.child(settings_field(
            "Preset name",
            self.settings.preset_name.clone(),
            p,
            cx,
        ));
        if !self.preset_busy {
            let mut actions = div()
                .flex()
                .flex_wrap()
                .gap(px(spacing::CONTROL_GAP))
                .child(button(
                    Icon::AddToList,
                    "Create preset",
                    p,
                    cx.listener(|t, _, _, c| t.manage_preset(PresetAction::Create, c)),
                ));
            if selected.is_some() {
                actions = actions
                    .child(button(
                        Icon::Rename,
                        "Rename",
                        p,
                        cx.listener(|t, _, _, c| t.manage_preset(PresetAction::Rename, c)),
                    ))
                    .child(button(
                        Icon::Confirm,
                        "Overwrite",
                        p,
                        cx.listener(|t, _, _, c| t.manage_preset(PresetAction::Overwrite, c)),
                    ))
                    .child(button(
                        Icon::Delete,
                        "Delete preset",
                        p,
                        cx.listener(|t, _, _, c| t.manage_preset(PresetAction::Delete, c)),
                    ));
            }
            controls = controls.child(actions);
        }
        if let (Some(action), Some(selected)) = (self.preset_confirmation, selected) {
            let deleting = action == PresetAction::Delete;
            controls = controls
                .child(div().text_sm().child(if deleting {
                    format!("Delete ‘{}’? This removes only the saved preset, not your current settings or imported files.", selected.name)
                } else {
                    format!("Overwrite ‘{}’ with all current settings? Its previous saved settings will be replaced.", selected.name)
                }))
                .child(div().flex().flex_wrap().gap(px(spacing::CONTROL_GAP))
                    .child(danger_button(if deleting { Icon::Delete } else { Icon::Confirm },
                        if deleting { "Confirm delete" } else { "Confirm overwrite" }, p,
                        cx.listener(move |t, _, _, c| t.manage_preset(action, c))))
                    .child(button(Icon::Cancel, "Cancel", p, cx.listener(|t, _, _, c| {
                        t.preset_confirmation = None;
                        c.notify();
                    }))));
        }
        if let Some(message) = &self.preset_message {
            controls = controls.child(div().text_sm().text_color(p.muted).child(message.clone()));
        }
        if self.preset_busy {
            controls = controls.child(
                div()
                    .text_sm()
                    .text_color(p.muted)
                    .child("Loading or saving presets…"),
            );
        }
        controls.into_any_element()
    }

    pub(crate) fn settings_panel(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let p = Palette::new(self.ui.scheme, self.ui.dark_mode);
        let panel = div()
            .id("settings-panel")
            .w_full().max_w(px(820.))
            .flex_1()
            .min_h_0()
            .min_w_0()
            .p(px(spacing::CONTENT))
            .flex()
            .flex_col()
            .gap(px(spacing::CONTENT))
            .overflow_y_scroll()
            .track_scroll(&self.scrollbars.settings.handle)
            .scrollbar_width(px(spacing::SCROLLBAR_GUTTER))
            .child(
                div()
                    .font_family(DISPLAY_FONT)
                    .text_lg()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("Settings"),
            )
            .child(div().text_sm().text_color(p.muted)
                .child("Set destinations and naming, then review the safety and media rules. Appearance and source alias save separately."))
            .child(settings_section("Presets", p))
            .child(div().text_xs().text_color(p.muted)
                .child("Built-in starting points replace import rules. Choose a saved preset to restore all settings."))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(spacing::CONTROL_GAP))
                    .child(chip(
                        "Everyday",
                        self.selected_preset.is_none() && self.preset.name == "Everyday", p,
                        cx.listener(|t, _, _, c| t.choose_preset(false, c)),
                    ))
                    .child(chip(
                        "Organized",
                        self.selected_preset.is_none() && self.preset.name == "Organized", p,
                        cx.listener(|t, _, _, c| t.choose_preset(true, c)),
                    )),
            )
            .child(self.saved_presets_controls(cx))
            .child(settings_section("Destinations", p))
            .child(settings_field(
                "Photo destination",
                self.settings.photo_root.clone(),
                p,
                cx,
            ))
            .child(div().text_xs().text_color(p.muted)
                .child("Destination roots are absolute paths, for example /home/you/Pictures. Folder templates create subfolders inside these roots."))
            .child(settings_template_field(
                "Photo folder template",
                self.settings.photo_folder.clone(),
                false,
                false,
                self.template_segment_target.as_ref(),
                p,
                cx,
            ))
            .child(settings_field(
                "Video destination",
                self.settings.video_root.clone(),
                p,
                cx,
            ))
            .child(settings_template_field(
                "Video folder template",
                self.settings.video_folder.clone(),
                false,
                true,
                self.template_segment_target.as_ref(),
                p,
                cx,
            ))
            .child(settings_section("File naming", p))
            .child(settings_template_field(
                "Filename template",
                self.settings.filename.clone(),
                true,
                false,
                self.template_segment_target.as_ref(),
                p,
                cx,
            ))
            .child(settings_section("Import safety", p))
            .child(div().text_xs().text_color(p.muted)
                .child("Click a rule to cycle its options. Verification checks copies; collision controls what happens when a destination exists."))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(spacing::CONTROL_GAP))
                    .child(
                        div()
                            .id("verification")
                            .cursor_pointer()
                            .p(px(spacing::CONTROL_GAP))
                            .bg(p.card).rounded_sm().border_1().border_color(p.border)
                            .hover(move |style| style.bg(p.selected))
                            .child(format!("Verification: {:?}", self.preset.verification))
                            .on_click(cx.listener(|t, _, _, c| t.cycle_verification(c))),
                    )
                    .child(
                        div()
                            .id("bundle-policy")
                            .cursor_pointer()
                            .p(px(spacing::CONTROL_GAP))
                            .bg(p.card).rounded_sm().border_1().border_color(p.border)
                            .hover(move |style| style.bg(p.selected))
                            .child(format!("Bundles: {:?}", self.preset.bundle_policy))
                            .on_click(cx.listener(|t, _, _, c| t.cycle_bundle_policy(c))),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(spacing::CONTROL_GAP))
                    .child(
                        div()
                            .id("grouping")
                            .cursor_pointer()
                            .p(px(spacing::CONTROL_GAP))
                            .bg(p.card).rounded_sm().border_1().border_color(p.border)
                            .hover(move |style| style.bg(p.selected))
                            .child(format!("Grouping: {:?}", self.preset.grouping))
                            .on_click(cx.listener(|t, _, _, c| t.cycle_grouping(c))),
                    )
                    .child(
                        div()
                            .id("collision")
                            .cursor_pointer()
                            .p(px(spacing::CONTROL_GAP))
                            .bg(p.card).rounded_sm().border_1().border_color(p.border)
                            .hover(move |style| style.bg(p.selected))
                            .child(format!("Collision: {:?}", self.preset.collision))
                            .on_click(cx.listener(|t, _, _, c| t.cycle_collision(c))),
                    ),
            )
            .child(settings_section("Media discovery", p))
            .child(div().text_xs().text_color(p.muted)
                .child("These rules determine which files appear when you scan a source."))
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
            .child(settings_section("Browsing", p))
            .child(div().text_xs().text_color(p.muted)
                .child("Choose whether a RAW and its JPEG sidecar appear as one item or as separate files."))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(spacing::CONTROL_GAP))
                    .child(chip(
                        "Merge RAW+JPEG",
                        self.ui.merge_raw_jpeg, p,
                        cx.listener(|t, _, _, c| t.set_merge_raw_jpeg(true, c)),
                    ))
                    .child(chip(
                        "Separate files",
                        !self.ui.merge_raw_jpeg, p,
                        cx.listener(|t, _, _, c| t.set_merge_raw_jpeg(false, c)),
                    )),
            )
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
                    .gap(px(spacing::TIGHT))
                    .child(
                        div()
                            .text_xs()
                            .text_color(p.muted)
                            .child("Timezone applied to capture times"),
                    )
                    .child(button(
                        Icon::Time,
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
                        ), p,
                        cx.listener(|t, _, _, c| {
                            t.timezone_menu_open = !t.timezone_menu_open;
                            c.notify()
                        }),
                    ))
                    .child(if self.timezone_menu_open {
                        div().relative().flex_shrink_0().max_h(px(200.)).flex().flex_col()
                            .child(div()
                            .id("timezone-menu")
                            .max_h(px(200.))
                            .overflow_y_scroll()
                            .track_scroll(&self.scrollbars.timezone.handle)
                            .scrollbar_width(px(spacing::SCROLLBAR_GUTTER))
                            .flex()
                            .flex_col()
                            .gap(px(spacing::TIGHT))
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
                            })))
                            .child(self.scrollbars.timezone.element(p.border, p.muted, p.accent))
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
                    .p(px(spacing::CONTROL_GAP))
                    .bg(p.card).rounded_sm().border_1().border_color(p.border)
                    .hover(move |style| style.bg(p.selected))
                    .child(format!("Backup required: {}", self.backup_required_choice))
                    .on_click(cx.listener(|t, _, _, c| t.toggle_backup_required(c))),
            )
            .child(settings_section("Source", p))
            .child(settings_field(
                "Current source alias (blank uses device name)",
                self.settings.source_alias.clone(), p, cx,
            ))
            .child(button(Icon::Confirm, "Save source alias", p,
                cx.listener(|t, _, _, c| t.save_source_alias(c))))
            .child(settings_section("Appearance", p))
            .child(div().text_xs().text_color(p.muted)
                .child("Color scheme and light or dark mode save immediately."))
            .child(div().flex().flex_wrap().gap(px(spacing::CONTROL_GAP))
                .children(ColorScheme::ALL.map(|scheme| chip(
                    scheme.label(), self.ui.scheme == scheme, p,
                    cx.listener(move |t, _, _, c| t.set_scheme(scheme, c)),
                ))))
            .child(div().flex().flex_wrap().gap(px(spacing::CONTROL_GAP))
                .child(chip("Light", !self.ui.dark_mode, p,
                    cx.listener(|t, _, _, c| { if t.ui.dark_mode { t.toggle_dark_mode(c); } })))
                .child(chip("Dark", self.ui.dark_mode, p,
                    cx.listener(|t, _, _, c| { if !t.ui.dark_mode { t.toggle_dark_mode(c); } }))))
            .into_any_element();
        div()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .flex()
            .flex_col()
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .child(panel)
                    .child(
                        self.scrollbars
                            .settings
                            .element(p.border, p.muted, p.accent),
                    ),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .border_t_1()
                    .border_color(p.border)
                    .px(px(spacing::CONTENT))
                    .py(px(spacing::CONTROL_GAP))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(spacing::CONTENT))
                    .child(
                        div()
                            .text_xs()
                            .text_color(p.muted)
                            .child("Review the import preview before copying."),
                    )
                    .child(primary_button(
                        Icon::Confirm,
                        "Apply settings",
                        p,
                        cx.listener(|t, _, _, c| t.apply_settings(c)),
                    )),
            )
            .into_any_element()
    }
}

use crate::*;

impl Browser {
    pub(crate) fn preview_panel(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let p = Palette::new(self.ui.scheme, self.ui.dark_mode);
        let Some(plan) = &self.plan else {
            return div()
                .flex_1()
                .p(px(spacing::CONTENT))
                .child(i18n::tr("No preview yet"))
                .into_any_element();
        };
        let deletion_scope = self
            .filesystem_root
            .as_ref()
            .and_then(|_| self.deletion_scope());
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
                        .child(i18n::format("Review import · {0} files", &[format!("{}", count)])))
                    .child(button(Icon::Back, "Back to media", p,  cx.listener(|t, _, _, c| { t.page = Page::Browser; c.notify() }))))
                .child(div().text_sm().text_color(p.muted)
                    .child(match (&self.last_import_result, blocked) {
                        (Some(_), _) => i18n::tr("Review the verified copies below."),
                        (None, 0) => i18n::tr("Check each final destination before copying."),
                        (None, _) => i18n::format("{0} blocked · Review the affected paths below", &[format!("{}", blocked)]),
                    })))
            .child(div().px(px(spacing::CONTENT)).py(px(spacing::CONTENT)).flex().flex_wrap().items_center().justify_between().gap(px(spacing::CONTENT)).border_b_1().border_color(p.border).bg(p.panel)
                .child(div().text_sm().child(match &self.last_import_result {
                    // After an import, counting planned copies as "to copy" misstates the outcome.
                    Some(result) => {
                        let copies = result.items.iter().flat_map(|item| &item.copies);
                        let verified = copies.clone().filter(|copy| copy.state == captureport_ingest::ImportItemState::Completed).count();
                        let failed = copies.filter(|copy| copy.state == captureport_ingest::ImportItemState::Failed).count();
                        let counts = [format!("{}", verified), format!("{}", failed), plan.preset_name.to_string(), i18n::tr(&format!("{:?}", plan.verification))];
                        // A cancellation has its own footer wording; do not report it as a failure.
                        i18n::format(if result.cancelled {
                            "Import cancelled · {0} verified · {1} failed · {2} preset · {3} verification"
                        } else {
                            "Import finished · {0} verified · {1} failed · {2} preset · {3} verification"
                        }, &counts)
                    }
                    None => i18n::format("{0} to copy · {1} skipped · {2} blocked · {3} preset · {4} verification", &[format!("{}", count.saturating_sub(blocked + skipped)), format!("{}", skipped), format!("{}", blocked), plan.preset_name.to_string(), i18n::tr(&format!("{:?}", plan.verification))]),
                }))
                .child(if blocked == 0 && count > 0 && !self.importing() && self.last_import_result.is_none() {
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
                if let Some(scope) = &deletion_scope {
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
                                            .child(scope.clone()),
                                    )
                                    .child(div().text_xs().text_color(p.muted).child(i18n::tr("This cannot be undone. Only files whose required copies verified are removed."))),
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
                                        i18n::format(" · Modified {0}", &[format_file_time(
                                                seconds,
                                                t.preset
                                                    .time_correction
                                                    .assumed_utc_offset_seconds
                                            ).to_string()])
                                    })
                                    .unwrap_or_default();
                                let destinations = items
                                    .iter()
                                    .flat_map(|item| &item.copies)
                                    .map(|c| {
                                        format!(
                                            "{}{} · {}",
                                            c.final_destination.display(),
                                            i18n::tr(if c.required { "" } else { " (optional)" }),
                                            i18n::tr(plan_status_label(c.status))
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
                                                i18n::tr(plan_status_label(status))
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
                    .child(i18n::format(
                        "Import history · {0} session(s)",
                        &[format!("{}", self.history.len())],
                    )),
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
                    .tab_index(0)
                    .focus(move |style| style.border_color(p.accent))
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
                                "{} · {} · {}",
                                format_session_time(&session.started_at),
                                entry.source_name,
                                i18n::tr(&format!("{:?}", session.status))
                            )),
                    )
                    .child(div().text_xs().child(i18n::format(
                        "{0} imported · {1} · {2}",
                        &[
                            format!("{}", imported),
                            format_size(bytes).to_string(),
                            destination.to_string(),
                        ],
                    ))),
            );
        }
        if self.history.is_empty() {
            panel = panel.child(div().p(px(spacing::CONTENT)).text_color(p.muted)
                .child(i18n::tr("No imports yet. Open a source, select media, then review an import to start your history.")));
        }
        if let Some(detail) = &self.session_detail {
            panel = panel.child(
                div()
                    .px(px(spacing::CONTENT))
                    .py(px(spacing::CONTENT))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child(i18n::format(
                        "Session {0} · {1} copy/copies",
                        &[
                            format!("{}", detail.session.id),
                            format!("{}", detail.imports.len()),
                        ],
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
                    .unwrap_or_else(|| i18n::tr("unknown"));
                panel = panel.child(
                    div()
                        .px(px(spacing::CONTENT))
                        .py(px(spacing::CONTROL_GAP))
                        .border_b_1()
                        .border_color(p.border)
                        .child(div().text_sm().child(format!(
                            "{destinations} · {}",
                            i18n::tr(&format!("{:?}", first.status))
                        )))
                        .child(div().text_xs().child(i18n::format(
                            "Modified {0} · Verification: {1} · {2}",
                            &[
                                modified.to_string(),
                                i18n::tr(first.verification_method.as_deref().unwrap_or("pending")),
                                first.error.as_deref().unwrap_or("").to_string(),
                            ],
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
                    .child(i18n::tr("Import recovery")),
            )
            .child(div().text_sm().child(i18n::format(
                "{0} unfinished session(s) · {1} incomplete file(s)",
                &[
                    format!("{}", self.incomplete_sessions.len()),
                    format!("{}", self.partial_files.len()),
                ],
            )));
        if self.incomplete_sessions.is_empty() && self.partial_files.is_empty() {
            panel = panel.child(div().mt(px(spacing::CONTENT)).p(px(spacing::CONTENT)).rounded_sm().bg(p.card)
                .child(i18n::tr("Nothing needs recovery. Interrupted imports and incomplete files will appear here.")));
        }
        for session in &self.incomplete_sessions {
            panel = panel.child(div().p(px(spacing::CONTENT)).bg(p.card).rounded_sm().child(
                i18n::format(
                    "Session {0} · started {1} · {2} recorded copy/copies",
                    &[
                        format!("{}", session.session.id),
                        format_session_time(&session.session.started_at),
                        format!("{}", session.imports.len()),
                    ],
                ),
            ));
        }
        if !self.incomplete_sessions.is_empty() {
            // Resuming needs the interrupted source connected and scanned; a
            // button that can only produce an error is worse than the reason.
            let resumable = self.catalog_source_id.is_some_and(|id| {
                self.incomplete_sessions
                    .iter()
                    .any(|session| session.session.source_id == id)
            }) && !self.scanning;
            panel = panel.child(if resumable {
                button(
                    Icon::AddToList,
                    "Review remaining files",
                    p,
                    cx.listener(|t, _, _, c| t.resume_from_current_source(c)),
                )
                .into_any_element()
            } else {
                div()
                    .text_sm()
                    .text_color(p.muted)
                    .child(i18n::tr(
                        "Reconnect the interrupted source and finish scanning to review its remaining files.",
                    ))
                    .into_any_element()
            });
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
                    .gap(px(spacing::CONTROL_GAP))
                    // A long path must ellipsize; otherwise it pushes the
                    // destructive confirmation controls out of the window.
                    .child(div().flex_1().min_w_0().truncate().child(format!(
                        "{} · {}",
                        partial.path.display(),
                        format_size(partial.size)
                    )))
                    .child(if self.armed_partial == Some(index) {
                        div()
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .gap(px(spacing::CONTROL_GAP))
                            .child(button(
                                Icon::Revert,
                                "Keep file",
                                p,
                                cx.listener(|t, _, _, c| t.cancel_clean_partial(c)),
                            ))
                            .child(danger_button_with_focus(
                                Icon::Delete,
                                "Confirm delete",
                                p,
                                &self.partial_confirm_focus,
                                cx.listener(move |t, _, w, c| t.clean_partial(index, w, c)),
                            ))
                            .into_any_element()
                    } else {
                        button(
                            Icon::Delete,
                            "Clean incomplete file",
                            p,
                            cx.listener(move |t, _, w, c| t.clean_partial(index, w, c)),
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
}

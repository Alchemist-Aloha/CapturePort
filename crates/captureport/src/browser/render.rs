use crate::*;

fn progress_fraction(bytes: u64, total: u64) -> f32 {
    if total == 0 {
        0.
    } else {
        (bytes as f64 / total as f64).clamp(0., 1.) as f32
    }
}

impl Browser {
    fn active_import_panel(&self, cx: &Context<Self>) -> Option<AnyElement> {
        if !self.importing {
            return None;
        }
        let p = Palette::new(self.ui.scheme, self.ui.dark_mode);
        let mut panel = div()
            .id("active-import")
            .flex_shrink_0()
            .px(px(spacing::CONTENT))
            .py(px(spacing::CONTROL_GAP))
            .border_t_1()
            .border_color(p.border)
            .bg(p.panel)
            .flex()
            .flex_col()
            .gap(px(spacing::TIGHT));
        if let Some(progress) = &self.progress {
            let overall = if progress.bytes_total == 0 {
                progress_fraction(progress.completed, progress.total)
            } else {
                progress_fraction(progress.bytes_copied, progress.bytes_total)
            };
            let current =
                progress_fraction(progress.current_file_bytes, progress.current_file_total);
            let name = self
                .state
                .item(progress.media_id)
                .map(|item| item.source_name.clone())
                .unwrap_or_else(|| {
                    progress
                        .destination
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned()
                });
            panel = panel
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .justify_between()
                        .gap(px(spacing::CONTROL_GAP))
                        .child(
                            div()
                                .text_sm()
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .child(format!(
                                    "Active import · {:.0}% · {}/{} copies verified",
                                    overall * 100.,
                                    progress.completed,
                                    progress.total
                                )),
                        )
                        .child(button(
                            Icon::Cancel,
                            "Cancel import",
                            p,
                            cx.listener(|t, _, w, c| t.cancel_import(&CancelImport, w, c)),
                        )),
                )
                .child(
                    div()
                        .h(px(4.))
                        .w_full()
                        .bg(p.border)
                        .child(div().h_full().w(gpui::relative(overall)).bg(p.accent)),
                )
                .child(div().text_xs().text_color(p.muted).child(format!(
                    "{} / {} transferred · {}/s",
                    format_size(progress.bytes_copied),
                    format_size(progress.bytes_total),
                    format_size(progress.bytes_per_second.max(0.) as u64)
                )))
                .child(div().w_full().text_sm().truncate().child(format!(
                    "Current copy: {name} · {} / {}",
                    format_size(progress.current_file_bytes),
                    format_size(progress.current_file_total)
                )))
                .child(
                    div()
                        .h(px(4.))
                        .w_full()
                        .bg(p.border)
                        .child(div().h_full().w(gpui::relative(current)).bg(p.accent)),
                )
                .child(
                    div()
                        .w_full()
                        .text_xs()
                        .text_color(p.muted)
                        .truncate()
                        .child(format!("To: {}", progress.destination.display())),
                );
        } else {
            panel = panel.child(div().text_sm().child("Preparing import…"));
        }
        Some(panel.into_any_element())
    }
}

impl Render for Browser {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Device removal can dismiss details without a mouse/keyboard event.
        // Return focus before the popup's focus tree disappears.
        if self.media_detail.is_none() && self.detail_focus.contains_focused(window, cx) {
            window.focus(&self.focus);
        }
        let p = Palette::new(self.ui.scheme, self.ui.dark_mode);
        let sidebar_width = if f32::from(window.bounds().size.width) < 760. {
            184.
        } else {
            232.
        };
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
        let detail = self.media_detail_popup(window, cx);
        let active_import = self.active_import_panel(cx);
        div()
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .bg(p.canvas)
            .font_family("Outfit")
            .text_size(px(14.))
            .text_color(p.text)
            .track_focus(&self.focus_handle(cx))
            .on_key_down(cx.listener(|t, event: &gpui::KeyDownEvent, window, cx| {
                if t.page == Page::Browser
                    && t.media_detail.is_none()
                    && event.keystroke.key == "tab"
                {
                    cx.stop_propagation();
                    if event.keystroke.modifiers.shift {
                        window.focus_prev();
                    } else {
                        window.focus_next();
                    }
                }
            }))
            .on_action(cx.listener(Self::select_none))
            .when(self.media_detail.is_none(), |view| {
                view.on_action(cx.listener(Self::open_folder))
                    .on_action(cx.listener(Self::open_demo))
                    .on_action(cx.listener(Self::select_all))
                    .on_action(cx.listener(Self::select_new))
                    .on_action(cx.listener(Self::mark_selected_imported))
                    .on_action(cx.listener(Self::import_selected))
                    .on_action(cx.listener(Self::cancel_import))
                    .on_action(cx.listener(Self::history))
                    .on_action(cx.listener(Self::reconcile))
                    .on_action(cx.listener(Self::clock))
                    .on_action(cx.listener(Self::cancel_reconcile))
                    .on_action(cx.listener(Self::discover_sources))
            })
            .child(
                div()
                    .h(px(48.))
                    .flex_shrink_0()
                    .border_b_1()
                    .border_color(p.border)
                    .px(px(spacing::CONTENT))
                    .flex()
                    .items_center()
                    .justify_between()
                    .bg(p.header)
                    .text_color(p.header_text)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(spacing::CONTENT))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(spacing::CONTROL_GAP))
                                    .child(crate::icons::logo_badge(28.))
                                    .child(
                                        div()
                                            .font_family(DISPLAY_FONT)
                                            .text_size(px(22.))
                                            .font_weight(gpui::FontWeight::BOLD)
                                            .child("CapturePort"),
                                    ),
                            )
                            .child(div().w(px(1.)).h(px(20.)).flex_shrink_0().bg(p.border))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(p.header_muted)
                                    .child("Photo and video ingest"),
                            ),
                    )
                    .child(button(
                        if self.ui.dark_mode {
                            Icon::LightMode
                        } else {
                            Icon::DarkMode
                        },
                        if self.ui.dark_mode {
                            "Light mode"
                        } else {
                            "Dark mode"
                        },
                        p,
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
            .children(active_import)
            .child(
                div()
                    .min_h(px(52.))
                    .flex_shrink_0()
                    .px(px(spacing::CONTENT))
                    .flex()
                    .items_center()
                    .justify_between()
                    .flex_wrap()
                    .border_t_1()
                    .border_color(p.border)
                    .bg(p.card)
                    .text_sm()
                    .child(div().min_w_0().truncate().text_color(p.muted).child(status))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(spacing::CONTENT))
                            .children((!self.state.is_empty()).then(|| {
                                format!(
                                    "{} items · {} selected · {}",
                                    self.state.len(),
                                    summary.count,
                                    format_size(summary.bytes)
                                )
                            }))
                            .child(
                                if self.page == Page::Browser
                                    && summary.count > 0
                                    && !self.importing
                                    && !self.planning
                                    && self.last_import_result.is_none()
                                {
                                    primary_button(
                                        Icon::Review,
                                        "Preview import",
                                        p,
                                        cx.listener(|t, _, w, c| {
                                            if t.plan.is_some() {
                                                t.page = Page::Preview;
                                                c.notify();
                                            } else {
                                                t.import_selected(&ImportSelected, w, c);
                                            }
                                        }),
                                    )
                                    .into_any_element()
                                } else {
                                    div().hidden().into_any_element()
                                },
                            ),
                    ),
            )
            .children(detail)
    }
}

#[cfg(test)]
mod tests {
    use super::progress_fraction;

    #[test]
    fn progress_is_bounded_and_handles_empty_files() {
        assert_eq!(progress_fraction(0, 0), 0.);
        assert_eq!(progress_fraction(5, 10), 0.5);
        assert_eq!(progress_fraction(20, 10), 1.);
        assert_eq!(progress_fraction(u64::MAX, u64::MAX), 1.);
    }
}

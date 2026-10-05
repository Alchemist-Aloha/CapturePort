use crate::*;

impl Render for Browser {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
                            .child(format!(
                                "{} items · {} selected · {}",
                                self.state.len(),
                                summary.count,
                                format_size(summary.bytes)
                            ))
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

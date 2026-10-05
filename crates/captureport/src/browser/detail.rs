use crate::*;
use captureport_core::{MediaItem, MetadataState, TimestampSource};
use std::{
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};

fn original_media_path(root: Option<&Path>, source_path: &str) -> PathBuf {
    root.map_or_else(|| PathBuf::from(source_path), |root| root.join(source_path))
}

fn original_file_url(root: Option<&Path>, source_path: &str, camera_port: Option<&str>) -> String {
    let original = original_media_path(root, source_path);
    let escaped: String = original
        .as_os_str()
        .as_bytes()
        .iter()
        .map(|&byte| {
            if byte.is_ascii_alphanumeric() || b"/-._~".contains(&byte) {
                char::from(byte).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect();
    if root.is_some() && original.is_absolute() {
        format!("file://{escaped}")
    } else if let Some(port) = camera_port {
        format!("gphoto2://[{port}]/{}", escaped.trim_start_matches('/'))
    } else {
        // A synthetic or disconnected source has no desktop-addressable URL.
        original.display().to_string()
    }
}

fn detail_rows(item: &MediaItem) -> Vec<(&'static str, String)> {
    let mut rows = vec![
        (
            "File size",
            format!("{} ({} bytes)", format_size(item.size), item.size),
        ),
        ("Import status", import_status_line(item)),
    ];
    match &item.metadata {
        MetadataState::Ready(metadata) => {
            rows.extend([
                (
                    "Capture time",
                    metadata
                        .capture_time
                        .clone()
                        .unwrap_or_else(|| "Unavailable".into()),
                ),
                (
                    "Timestamp source",
                    match metadata.timestamp_source {
                        Some(TimestampSource::ExifOriginal) => "EXIF original",
                        Some(TimestampSource::QuickTime) => "Video container",
                        Some(TimestampSource::Camera) => "Camera-reported",
                        Some(TimestampSource::Filesystem) => "Filesystem",
                        None => "Unavailable",
                    }
                    .into(),
                ),
                (
                    "Dimensions",
                    match (metadata.width, metadata.height) {
                        (Some(width), Some(height)) => format!("{width} × {height}"),
                        _ => "Unavailable".into(),
                    },
                ),
                (
                    "Camera",
                    [
                        metadata.camera_make.as_deref(),
                        metadata.camera_model.as_deref(),
                    ]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
                    .join(" "),
                ),
            ]);
            if let Some(time) = &metadata.filesystem_time {
                rows.push(("Filesystem time", time.clone()));
            }
            if let Some(lens) = &metadata.lens {
                rows.push(("Lens", lens.clone()));
            }
            if let Some(serial) = &metadata.camera_serial {
                rows.push(("Camera serial", serial.clone()));
            }
            if let Some(orientation) = metadata.orientation {
                rows.push(("Orientation", format!("{orientation:?}")));
            }
            if let Some(millis) = metadata.duration_millis {
                rows.push(("Duration", format!("{:.3} seconds", millis as f64 / 1000.)));
            }
            if let Some((lat, lon)) = metadata.gps_e7 {
                rows.push((
                    "GPS",
                    format!(
                        "{:.7}, {:.7}",
                        lat as f64 / 10_000_000.,
                        lon as f64 / 10_000_000.
                    ),
                ));
            }
        }
        MetadataState::Pending => rows.push(("Metadata", "Loading…".into())),
        MetadataState::Failed(error) => rows.push(("Metadata", format!("Unavailable: {error}"))),
    }
    if let Some(prior) = &item.prior_import {
        rows.push(("Prior import destination", prior.destination.clone()));
    }
    rows
}

struct DetailTooltip(Palette);

impl Render for DetailTooltip {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .px(px(spacing::CONTROL_GAP))
            .py(px(spacing::TIGHT))
            .bg(self.0.card)
            .text_color(self.0.text)
            .border_1()
            .border_color(self.0.border)
            .rounded_sm()
            .text_sm()
            .child("View media details")
    }
}

impl Browser {
    pub(crate) fn media_detail_trigger(
        &self,
        id: MediaId,
        p: Palette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .id(("media-details", id.0))
            .focusable()
            .tab_index(0)
            .size(px(spacing::CONTROL_HEIGHT))
            .flex_shrink_0()
            .rounded_sm()
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .border_1()
            .border_color(gpui::rgba(0))
            .text_color(p.muted)
            .hover(move |style| style.bg(p.selected).text_color(p.text))
            .focus(move |style| style.border_color(p.accent).bg(p.selected))
            .tooltip(move |_, cx| cx.new(|_| DetailTooltip(p)).into())
            .on_click(cx.listener(move |t, _, w, c| {
                c.stop_propagation();
                t.open_media_detail(id, w, c);
            }))
            .on_key_down(cx.listener(move |t, event: &gpui::KeyDownEvent, w, c| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    c.stop_propagation();
                    t.open_media_detail(id, w, c);
                }
            }))
            .child(icons::icon(Icon::Review, p.text))
            .into_any_element()
    }

    fn copy_detail_url(&mut self, url: &str, cx: &mut Context<Self>) {
        cx.write_to_clipboard(gpui::ClipboardItem::new_string(url.to_string()));
        self.detail_url_copied = true;
        cx.notify();
    }

    pub(crate) fn open_media_detail(
        &mut self,
        id: MediaId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.state.item(id).is_none() {
            return;
        }
        self.media_detail = Some(id);
        self.detail_url_copied = false;
        self.scrollbars.detail = Default::default();
        window.focus(&self.detail_focus);
        cx.notify();
    }

    pub(crate) fn close_media_detail(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.media_detail = None;
        self.detail_url_copied = false;
        window.focus(&self.focus);
        cx.notify();
    }

    pub(crate) fn media_detail_popup(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let id = self.media_detail?;
        let item = self.state.item(id)?;
        let p = Palette::new(self.ui.scheme, self.ui.dark_mode);
        let original = original_media_path(self.filesystem_root.as_deref(), &item.source_path);
        let camera_port = self
            .discovered_sources
            .iter()
            .find_map(|source| match source {
                captureport_gphoto::discovery::DiscoveredSource::Ptp(camera)
                    if self
                        .state
                        .source
                        .as_ref()
                        .and_then(|source| source.stable_id.as_deref())
                        == Some(camera.stable_id.as_str()) =>
                {
                    Some(camera.port.as_str())
                }
                _ => None,
            });
        let file_url = original_file_url(
            self.filesystem_root.as_deref(),
            &item.source_path,
            camera_port,
        );
        let source_name = self
            .source_alias
            .clone()
            .or_else(|| {
                self.state
                    .source
                    .as_ref()
                    .and_then(|source| source.display_name.clone())
            })
            .unwrap_or_else(|| "Source".into());
        let wide = f32::from(window.bounds().size.width) >= 860.;
        let compact = f32::from(window.bounds().size.width) < 600.;
        let preview_height = if wide {
            260.
        } else {
            (f32::from(window.bounds().size.height) * 0.24).clamp(100., 180.)
        };
        let mut preview = div()
            .flex()
            .flex_col()
            .gap(px(spacing::CONTROL_GAP))
            .min_w_0()
            .when(wide, |view| view.w(px(280.)).flex_shrink_0())
            .when(!wide, |view| view.w_full());
        let image_well = div()
            .w_full()
            .h(px(preview_height))
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(p.placeholder)
            .rounded_sm()
            .overflow_hidden();
        preview = preview
            .child(
                if let Some(path) = self.thumbnail_paths.get(&self.preview_id(id)) {
                    image_well
                        .child(
                            img(path.clone())
                                .h(px(preview_height))
                                .max_w_full()
                                .object_fit(gpui::ObjectFit::Contain),
                        )
                        .into_any_element()
                } else {
                    image_well
                        .child(div().text_color(p.muted).child("Preview unavailable"))
                        .into_any_element()
                },
            )
            .child(
                div()
                    .id("detail-filename")
                    .w_full()
                    .min_w_0()
                    .overflow_x_scroll()
                    .text_sm()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child(item.source_name.clone()),
            )
            .child(div().text_xs().text_color(p.muted).child(format!(
                "{} · {}",
                media_type_badge(item.media_type).1,
                format_size(item.size)
            )));

        let click_url = file_url.clone();
        let mut facts = div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(px(spacing::CONTENT))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(spacing::TIGHT))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(spacing::CONTROL_GAP))
                            .child(div().text_xs().text_color(p.muted).child("Original file"))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(spacing::TIGHT))
                                    .text_xs()
                                    .text_color(if self.detail_url_copied {
                                        p.accent
                                    } else {
                                        p.muted
                                    })
                                    .when(self.detail_url_copied, |view| {
                                        view.child(icons::icon_sized(
                                            Icon::Confirm,
                                            icons::ICON_SIZE_COMPACT,
                                            p.accent,
                                        ))
                                    })
                                    .child(if self.detail_url_copied {
                                        "URL copied"
                                    } else {
                                        "Click to copy URL"
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .id("original-file-copy")
                            .focusable()
                            .tab_index(0)
                            .min_w_0()
                            .min_h(px(spacing::CONTROL_HEIGHT))
                            .flex()
                            .items_center()
                            .px(px(spacing::CONTROL_GAP))
                            .py(px(spacing::TIGHT))
                            .rounded_sm()
                            .border_1()
                            .border_color(p.border)
                            .bg(p.panel)
                            .cursor_pointer()
                            .hover(move |style| style.bg(p.selected))
                            .focus(move |style| style.border_color(p.accent))
                            .on_click(cx.listener(move |t, _, _, c| {
                                c.stop_propagation();
                                t.copy_detail_url(&click_url, c);
                            }))
                            .on_key_down(cx.listener(move |t, event: &gpui::KeyDownEvent, _, c| {
                                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                    c.stop_propagation();
                                    t.copy_detail_url(&file_url, c);
                                }
                            }))
                            .child(
                                div()
                                    .id("original-file-path")
                                    .flex_1()
                                    .min_w_0()
                                    .overflow_x_scroll()
                                    .child(original.display().to_string()),
                            ),
                    ),
            );
        let mut rows = vec![("Source", source_name)];
        rows.extend(detail_rows(item));
        facts = facts.children(rows.into_iter().map(|(label, value)| {
            div()
                .flex_shrink_0()
                .min_w_0()
                .flex()
                .gap(px(spacing::CONTROL_GAP))
                .py(px(spacing::TIGHT))
                .border_b_1()
                .border_color(p.border)
                .when(compact, |view| view.flex_col())
                .child(
                    div()
                        .flex_shrink_0()
                        .text_xs()
                        .text_color(p.muted)
                        .when(!compact, |view| view.w(px(120.)))
                        .child(label),
                )
                .child(
                    div()
                        .id(label)
                        .flex_1()
                        .min_w_0()
                        .overflow_x_scroll()
                        .text_sm()
                        .child(if value.is_empty() {
                            "Unavailable".into()
                        } else {
                            value
                        }),
                )
        }));
        let body = div()
            .id("media-detail-content")
            .flex_1()
            .min_h_0()
            .min_w_0()
            .p(px(spacing::CONTENT))
            .overflow_y_scroll()
            .track_scroll(&self.scrollbars.detail.handle)
            .scrollbar_width(px(spacing::SCROLLBAR_GUTTER))
            .child(
                div()
                    .flex()
                    .gap(px(spacing::SECTION))
                    .min_w_0()
                    .when(!wide, |view| view.flex_col())
                    .child(preview)
                    .child(facts),
            );
        Some(
            div()
                .id("media-detail-backdrop")
                .absolute()
                .inset_0()
                .occlude()
                .flex()
                .items_center()
                .justify_center()
                .p(px(spacing::CONTENT))
                .bg(gpui::rgba(0x00000099))
                .on_click(cx.listener(|t, _, w, c| t.close_media_detail(w, c)))
                .child(
                    div()
                        .id("media-detail-popup")
                        .occlude()
                        .tab_group()
                        .tab_index(0)
                        .tab_stop(false)
                        .track_focus(&self.detail_focus)
                        .on_key_down(|event, window, cx| {
                            if event.keystroke.key == "tab" {
                                cx.stop_propagation();
                                if event.keystroke.modifiers.shift {
                                    window.focus_prev();
                                } else {
                                    window.focus_next();
                                }
                            }
                        })
                        .w_full()
                        .max_w(px(880.))
                        .h(px((f32::from(window.bounds().size.height)
                            - spacing::CONTENT * 2.)
                            .clamp(120., if wide { 560. } else { 640. })))
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .rounded_sm()
                        .border_1()
                        .border_color(p.border)
                        .bg(p.card)
                        .on_click(|_, _, cx| cx.stop_propagation())
                        .child(
                            div()
                                .flex_shrink_0()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap(px(spacing::CONTENT))
                                .p(px(spacing::CONTENT))
                                .border_b_1()
                                .border_color(p.border)
                                .child(
                                    div()
                                        .font_family(DISPLAY_FONT)
                                        .text_lg()
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .child("Media details"),
                                )
                                .child(
                                    div()
                                        .id("media-detail-close")
                                        .focusable()
                                        .tab_index(1)
                                        .rounded_sm()
                                        .border_1()
                                        .border_color(gpui::rgba(0))
                                        .focus(move |style| style.border_color(p.accent))
                                        .on_key_down(cx.listener(
                                            |t, event: &gpui::KeyDownEvent, w, c| {
                                                if matches!(
                                                    event.keystroke.key.as_str(),
                                                    "enter" | "space"
                                                ) {
                                                    c.stop_propagation();
                                                    t.close_media_detail(w, c);
                                                }
                                            },
                                        ))
                                        .child(button(
                                            Icon::Cancel,
                                            "Close",
                                            p,
                                            cx.listener(|t, _, w, c| t.close_media_detail(w, c)),
                                        )),
                                ),
                        )
                        .child(
                            div()
                                .relative()
                                .flex_1()
                                .min_h_0()
                                .flex()
                                .flex_col()
                                .child(body)
                                .child(self.scrollbars.detail.element(p.border, p.muted, p.accent)),
                        ),
                )
                .into_any_element(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_file_urls_escape_local_and_camera_paths() {
        assert_eq!(
            original_file_url(Some(Path::new("/media/My card")), "DCIM/a #1%.jpg", None),
            "file:///media/My%20card/DCIM/a%20%231%25.jpg"
        );
        assert_eq!(
            original_file_url(Some(Path::new("/media/card")), "写真.jpg", None),
            "file:///media/card/%E5%86%99%E7%9C%9F.jpg"
        );
        assert_eq!(
            original_file_url(None, "/store_0001/DCIM/a ?.jpg", Some("usb:001,055")),
            "gphoto2://[usb:001,055]/store_0001/DCIM/a%20%3F.jpg"
        );
        assert_eq!(
            original_file_url(None, "DCIM/demo.jpg", None),
            "DCIM/demo.jpg"
        );
    }

    #[test]
    fn details_use_original_source_paths_and_do_not_invent_missing_dates() {
        let root = Path::new("/media/card");
        assert_eq!(
            original_media_path(Some(root), "DCIM/Camera/photo.jpg"),
            Path::new("/media/card/DCIM/Camera/photo.jpg")
        );
        assert_eq!(
            original_media_path(Some(root), "photo.jpg").parent(),
            Some(root)
        );
        assert_eq!(
            original_media_path(None, "/store_0001/DCIM/Camera/photo.jpg").parent(),
            Some(Path::new("/store_0001/DCIM/Camera"))
        );
        let mut item = MediaItem::new(MediaId(1), SourceId(1), "写真.jpg", 42);
        item.metadata = MetadataState::Ready(Default::default());
        assert!(detail_rows(&item).contains(&("Capture time", "Unavailable".into())));
        item.metadata = MetadataState::Failed("Unreadable EXIF".into());
        assert!(
            detail_rows(&item)
                .iter()
                .any(|(label, text)| *label == "Metadata" && text.contains("Unreadable EXIF"))
        );
        assert_eq!(item.source_name, "写真.jpg");
    }
}

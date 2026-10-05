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
        ("Filename", item.source_name.clone()),
        ("Media type", media_type_badge(item.media_type).1.into()),
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

impl Browser {
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
        self.scrollbars.detail = Default::default();
        window.focus(&self.detail_focus);
        cx.notify();
    }

    pub(crate) fn close_media_detail(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.media_detail = None;
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
        let mut rows = vec![
            ("Source", source_name),
            ("Original file", original.display().to_string()),
        ];
        rows.extend(detail_rows(item));
        let mut body = div()
            .id("media-detail-content")
            .flex_1()
            .min_h_0()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(px(spacing::CONTENT))
            .p(px(spacing::CONTENT))
            .overflow_y_scroll()
            .track_scroll(&self.scrollbars.detail.handle)
            .scrollbar_width(px(spacing::SCROLLBAR_GUTTER));
        if let Some(path) = self.thumbnail_paths.get(&self.preview_id(id)) {
            body = body.child(
                div()
                    .w_full()
                    .h(px(220.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(p.placeholder)
                    .rounded_sm()
                    .overflow_hidden()
                    .child(
                        img(path.clone())
                            .h(px(220.))
                            .max_w_full()
                            .object_fit(gpui::ObjectFit::Contain),
                    ),
            );
        } else {
            body = body.child(div().text_color(p.muted).child("Preview unavailable"));
        }
        body = body.children(rows.into_iter().map(|(label, value)| {
            div()
                .flex_shrink_0()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(spacing::TIGHT))
                .child(
                    div()
                        .text_xs()
                        .text_color(p.muted)
                        .child(if label == "Original file" {
                            "Original file · click to copy URL"
                        } else {
                            label
                        }),
                )
                .child(
                    div()
                        .id(label)
                        .min_w_0()
                        .overflow_x_scroll()
                        .when(label == "Original file", |view| {
                            let file_url = file_url.clone();
                            view.cursor_pointer()
                                .rounded_sm()
                                .hover(move |style| style.bg(p.selected))
                                .on_click(move |_, _, cx| {
                                    cx.stop_propagation();
                                    cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                        file_url.clone(),
                                    ));
                                })
                        })
                        .child(if value.is_empty() {
                            "Unavailable".into()
                        } else {
                            value
                        }),
                )
        }));
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
                        .track_focus(&self.detail_focus)
                        .w_full()
                        .max_w(px(720.))
                        .h(px((f32::from(window.bounds().size.height)
                            - spacing::CONTENT * 2.)
                            .clamp(120., 800.)))
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
                                .child(button(
                                    Icon::Cancel,
                                    "Close",
                                    p,
                                    cx.listener(|t, _, w, c| t.close_media_detail(w, c)),
                                )),
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

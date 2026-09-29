use gpui::{AnyElement, ObjectFit, Rgba, StyledImage};
use icons::{Icon, icon};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum ColorScheme {
    #[default]
    Pine,
    Darkroom,
    Graphite,
    Ink,
}
impl ColorScheme {
    const ALL: [ColorScheme; 4] = [Self::Pine, Self::Darkroom, Self::Graphite, Self::Ink];
    fn label(self) -> &'static str {
        match self {
            Self::Pine => "Pine",
            Self::Darkroom => "Darkroom",
            Self::Graphite => "Graphite",
            Self::Ink => "Ink",
        }
    }
}

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
    header: Rgba,
    header_text: Rgba,
    header_muted: Rgba,
    primary_bg: Rgba,
    primary_hover: Rgba,
    primary_text: Rgba,
    ghost_bg: Rgba,
    ghost_hover: Rgba,
    ghost_border: Rgba,
}
impl Palette {
    fn new(scheme: ColorScheme, dark: bool) -> Self {
        match (scheme, dark) {
            (ColorScheme::Pine, true) => Self {
                canvas: rgb(0x171a19), panel: rgb(0x202422), card: rgb(0x282d2a),
                border: rgb(0x424a45), muted: rgb(0xaeb8b1), placeholder: rgb(0x303632),
                selected: rgb(0x303e35), accent: rgb(0x9ccbad), danger: rgb(0x613b35),
                text: rgb(0xe8ede9),
                header: rgb(0x202422), header_text: rgb(0xe8ede9), header_muted: rgb(0xaeb8b1),
                primary_bg: rgb(0x9ccbad), primary_hover: rgb(0xa1e8bf), primary_text: rgb(0x171a19),
                ghost_bg: rgb(0x282d2a), ghost_hover: rgb(0x303632),
                ghost_border: rgb(0x424a45),
            },
            (ColorScheme::Pine, false) => Self {
                canvas: rgb(0xfafbf9), panel: rgb(0xf0f2ef), card: rgb(0xffffff),
                border: rgb(0xd2d8d2), muted: rgb(0x58625b), placeholder: rgb(0xe8ece7),
                selected: rgb(0xe1eae1), accent: rgb(0x276f51), danger: rgb(0xf5e3dc),
                text: rgb(0x242b26),
                header: rgb(0xf0f2ef), header_text: rgb(0x242b26), header_muted: rgb(0x58625b),
                primary_bg: rgb(0x247c66), primary_hover: rgb(0x195d4c), primary_text: rgb(0xffffff),
                ghost_bg: rgb(0xffffff), ghost_hover: rgb(0xe8ece7),
                ghost_border: rgb(0xd2d8d2),
            },
            (ColorScheme::Darkroom, true) => Self {
                canvas: rgb(0x1c1a18), panel: rgb(0x25221f), card: rgb(0x2f2b26),
                border: rgb(0x4b443c), muted: rgb(0xbfb3a5), placeholder: rgb(0x37322c),
                selected: rgb(0x42392e), accent: rgb(0xe8b06a), danger: rgb(0x6b3630),
                text: rgb(0xf2ece2),
                header: rgb(0x25221f), header_text: rgb(0xf2ece2), header_muted: rgb(0xbfb3a5),
                primary_bg: rgb(0xe8b06a), primary_hover: rgb(0xf0c489), primary_text: rgb(0x1c1a18),
                ghost_bg: rgb(0x2f2b26), ghost_hover: rgb(0x37322c),
                ghost_border: rgb(0x4b443c),
            },
            (ColorScheme::Darkroom, false) => Self {
                canvas: rgb(0xfaf9f6), panel: rgb(0xf1eee8), card: rgb(0xfffefa),
                border: rgb(0xd9d3c8), muted: rgb(0x655e53), placeholder: rgb(0xeae6de),
                selected: rgb(0xeee4d6), accent: rgb(0xa4653a), danger: rgb(0xf7e6da),
                text: rgb(0x2b2119),
                header: rgb(0xf1eee8), header_text: rgb(0x2b2119), header_muted: rgb(0x655e53),
                primary_bg: rgb(0xa4653a), primary_hover: rgb(0x8a4f2b), primary_text: rgb(0xfffefa),
                ghost_bg: rgb(0xfffefa), ghost_hover: rgb(0xeae6de),
                ghost_border: rgb(0xd9d3c8),
            },
            (ColorScheme::Graphite, true) => Self {
                canvas: rgb(0x191b1e), panel: rgb(0x22252a), card: rgb(0x2b2e34),
                border: rgb(0x464b53), muted: rgb(0xb4bbc5), placeholder: rgb(0x33373e),
                selected: rgb(0x333d49), accent: rgb(0x8fb4d9), danger: rgb(0x5e3835),
                text: rgb(0xf1f3f5),
                header: rgb(0x22252a), header_text: rgb(0xf1f3f5), header_muted: rgb(0xb4bbc5),
                primary_bg: rgb(0x8fb4d9), primary_hover: rgb(0xa6c6e6), primary_text: rgb(0x191b1e),
                ghost_bg: rgb(0x2b2e34), ghost_hover: rgb(0x33373e),
                ghost_border: rgb(0x464b53),
            },
            (ColorScheme::Graphite, false) => Self {
                canvas: rgb(0xf9fafb), panel: rgb(0xf0f2f4), card: rgb(0xffffff),
                border: rgb(0xd2d7de), muted: rgb(0x566068), placeholder: rgb(0xe6e9ee),
                selected: rgb(0xdfe6ef), accent: rgb(0x2f5f8f), danger: rgb(0xf4e3e0),
                text: rgb(0x1c2126),
                header: rgb(0xf0f2f4), header_text: rgb(0x1c2126), header_muted: rgb(0x566068),
                primary_bg: rgb(0x2f5f8f), primary_hover: rgb(0x24496e), primary_text: rgb(0xffffff),
                ghost_bg: rgb(0xffffff), ghost_hover: rgb(0xe6e9ee),
                ghost_border: rgb(0xd2d7de),
            },
            (ColorScheme::Ink, true) => Self {
                canvas: rgb(0x181b23), panel: rgb(0x222630), card: rgb(0x2b303d),
                border: rgb(0x444d62), muted: rgb(0xb3bdd1), placeholder: rgb(0x323a4b),
                selected: rgb(0x333e58), accent: rgb(0x86a6ff), danger: rgb(0x5c3630),
                text: rgb(0xeef1f8),
                header: rgb(0x222630), header_text: rgb(0xeef1f8), header_muted: rgb(0xb3bdd1),
                primary_bg: rgb(0x86a6ff), primary_hover: rgb(0xa3bcff), primary_text: rgb(0x181b23),
                ghost_bg: rgb(0x2b303d), ghost_hover: rgb(0x323a4b),
                ghost_border: rgb(0x444d62),
            },
            (ColorScheme::Ink, false) => Self {
                canvas: rgb(0xfafafd), panel: rgb(0xf0f1f6), card: rgb(0xffffff),
                border: rgb(0xd5d7e0), muted: rgb(0x5a6272), placeholder: rgb(0xe8eaf1),
                selected: rgb(0xe3e6f2), accent: rgb(0x2b48b8), danger: rgb(0xf6e4df),
                text: rgb(0x1a1e2b),
                header: rgb(0xf0f1f6), header_text: rgb(0x1a1e2b), header_muted: rgb(0x5a6272),
                primary_bg: rgb(0x2b48b8), primary_hover: rgb(0x1f3691), primary_text: rgb(0xffffff),
                ghost_bg: rgb(0xffffff), ghost_hover: rgb(0xe8eaf1),
                ghost_border: rgb(0xd5d7e0),
            },
        }
    }
}

fn thumbnail_columns(available: f32, target: f32) -> usize {
    ((available + spacing::GRID_GAP) / (target + spacing::GRID_GAP)).round().max(1.) as usize
}

fn thumbnail_layout(window_width: f32, window_height: f32, sidebar_width: f32, size: u8, view_options: bool, bundle_open: bool) -> (usize, f32) {
    let available = (window_width - sidebar_width - 2. * spacing::CONTENT - spacing::SCROLLBAR_GUTTER).max(1.);
    let target = [172., 210., 250., 300., 360.][size.min(4) as usize];
    let columns = thumbnail_columns(available, target);
    let card_width = ((available - spacing::GRID_GAP * (columns - 1) as f32) / columns as f32).max(1.);
    let extra_chrome = if view_options { 86. } else { 0. } + if bundle_open { 240. } else { 0. };
    let viewport_height = (window_height - 48. - 52. - 208. - spacing::MEDIA_ROW_CHROME - extra_chrome).max(1.);
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

/// The tile status line. When the catalog named the prior import behind the
/// classification, say which one: an unexplained "possible duplicate" is the
/// state this product must never leave the user guessing about.
fn import_status_line(item: &captureport_core::MediaItem) -> String {
    if item.manually_marked_imported {
        return "✓ Imported · marked manually".into();
    }
    let label = import_status_label(&item.import_status);
    match &item.prior_import {
        Some(prior) => {
            // Stored RFC 3339; the date is what fits on a tile.
            let day = prior.imported_at.get(..10).unwrap_or(prior.imported_at.as_str());
            format!("{label} · session {} · {day}", prior.session_id)
        }
        None => label.to_string(),
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

/// The tile's media-type badge: a real Material icon plus its label, replacing
/// the leading Unicode marks that stood in for an icon system.
fn media_type_badge(media_type: captureport_core::MediaType) -> (crate::icons::Icon, &'static str) {
    use captureport_core::MediaType;
    use crate::icons::Icon;
    match media_type {
        MediaType::Video => (Icon::Video, "VIDEO"),
        MediaType::Raw => (Icon::Photo, "RAW"),
        MediaType::Jpeg => (Icon::Photo, "JPEG"),
        MediaType::Heif => (Icon::Photo, "HEIF"),
        MediaType::Png => (Icon::Photo, "PNG"),
        MediaType::Tiff => (Icon::Photo, "TIFF"),
        MediaType::Sidecar => (Icon::File, "SIDECAR"),
        MediaType::Unknown => (Icon::File, "FILE"),
    }
}

fn bundle_type_label(
    media_types: impl Iterator<Item = captureport_core::MediaType>,
) -> (crate::icons::Icon, &'static str) {
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
    let label = if raw && jpeg {
        "RAW+JPEG"
    } else if video && sidecar {
        "VIDEO+SIDECAR"
    } else {
        "BUNDLE"
    };
    (crate::icons::Icon::Bundle, label)
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
    use super::{media_type_badge, segment_help_label, thumbnail_columns, thumbnail_layout};
    use crate::icons::Icon;

    #[test]
    fn segment_help_buttons_have_distinct_ids_and_toggle_labels() {
        let fields = ["Photo folder template", "Video folder template", "Filename template"];
        for expanded in [false, true] {
            let labels = fields.map(|field| segment_help_label(field, expanded));
            assert_eq!(labels.iter().collect::<std::collections::HashSet<_>>().len(), 3);
        }
    }

    #[test]
    fn grid_reflows_without_reserving_more_than_available_width() {
        for available in [120., 180., 320., 580., 900.] {
            for target in [172., 210., 250., 300., 360.] {
                let columns = thumbnail_columns(available, target);
                assert!(columns >= 1);
                let card_width =
                    (available - (columns - 1) as f32 * super::spacing::GRID_GAP) / columns as f32;
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
        assert_eq!(media_type_badge(MediaType::Video), (Icon::Video, "VIDEO"));
        assert_eq!(media_type_badge(MediaType::Raw), (Icon::Photo, "RAW"));
        assert_eq!(media_type_badge(MediaType::Jpeg), (Icon::Photo, "JPEG"));
        assert_eq!(media_type_badge(MediaType::Sidecar), (Icon::File, "SIDECAR"));
        assert_ne!(media_type_badge(MediaType::Video).0, media_type_badge(MediaType::Jpeg).0);
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
        // A filesystem source reads bytes from a path the thumbnail pipeline can
        // open directly. Every other source exposes its preview through the
        // `MediaSource`, so it must take the source-preview route instead.
        if source_identity.source_type != SourceType::Filesystem {
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
            source_id: format!("{}:{}", source_identity.stable_id.as_deref().unwrap_or(""), root.display()),
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
                "Open folder…",
                p,
                cx.listener(|t, _, w, c| t.open_folder(&OpenFolder, w, c)),
            ))
            .child(button(
                Icon::Refresh,
                if self.discovering { "Scanning devices…" } else { "Refresh devices" }, p,
                cx.listener(|t, _, w, c| t.discover_sources(&DiscoverSources, w, c)),
            ))
            .child(if self.discovering { "Looking for connected cameras and cards…" } else { "" });
        for (index, card) in self.unmounted_cards.iter().enumerate() {
            panel = panel.child(div().flex().flex_col().gap(px(spacing::TIGHT))
                .child(div().text_sm().child(format!("Card · {} · Not mounted", card.label)))
                .child(button(Icon::Storage, if self.mounting.as_ref() == Some(&card.path) { "Mounting…" } else { "Mount" }, p,
                    cx.listener(move |t, _, _, c| t.mount_card(index, c)))));
        }
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
                    .px(px(spacing::CONTENT)).py(px(spacing::TIGHT))
                    .flex().items_center().text_sm()
                    .min_h(px(spacing::CONTROL_HEIGHT))
                    .flex_shrink_0()
                    .rounded_sm()
                    .border_1().border_color(p.border)
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
        panel = panel
            .child(
                div()
                    .flex_shrink_0()
                    .text_xs()
                    .text_color(p.muted)
                    .truncate()
                    .child(format!("Current: {source}")),
            )
            .child(
                div()
                    .mt(px(spacing::SECTION - spacing::CONTROL_GAP))
                    .text_xs()
                    .text_color(p.muted)
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("Workspace"),
            )
            .child(sidebar_nav(Icon::Photo, "Browse media", self.page == Page::Browser, p,
                cx.listener(|t, _, _, c| {
                    t.page = Page::Browser;
                    c.notify()
                }),
            ))
            .child(if self.plan.is_some() {
                sidebar_nav(Icon::Review, "Review import", self.page == Page::Preview, p,
                    cx.listener(|t, _, _, c| { t.page = Page::Preview; c.notify() }),
                ).into_any_element()
            } else { div().hidden().into_any_element() })
            .child(sidebar_nav(Icon::History, "History", self.page == Page::History, p,
                cx.listener(|t, _, w, c| t.history(&ShowHistory, w, c)),
            ))
            .child(sidebar_nav(Icon::Revert, "Recovery", self.page == Page::Recovery, p,
                cx.listener(|t, _, _, c| t.show_recovery(c)),
            ))
            .child(sidebar_nav(Icon::Settings, "Settings", self.page == Page::Settings, p,
                cx.listener(|t, _, _, c| t.show_settings(c)),
            ))
            .child(div().mt(px(spacing::SECTION - spacing::CONTROL_GAP)).text_sm().font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(p.muted).child("Tools"))
            .child(button(Icon::Catalog, "Reconcile library", p,
                cx.listener(|t, _, w, c| t.reconcile(&ReconcileLibrary, w, c)),
            ))
            .child(if self.reconcile_cancellation.is_some() {
                button(Icon::Cancel, "Cancel library scan", p,
                    cx.listener(|t, _, w, c| t.cancel_reconcile(&CancelReconcile, w, c)),
                ).into_any_element()
            } else { div().hidden().into_any_element() })
            .child(button(
                Icon::Delete, "Clear thumbnails", p,
                cx.listener(|t, _, _, c| t.clear_thumbnail_cache(c)),
            ))
            .child(button(
                Icon::Play, "10k-item demo", p,
                cx.listener(|t, _, w, c| t.open_demo(&OpenDemo, w, c)),
            ))
            .child(if self.importing {
                button(Icon::Cancel, "Cancel import", p,  cx.listener(|t, _, w, c| t.cancel_import(&CancelImport, w, c)))
                    .into_any_element()
            } else { div().hidden().into_any_element() })
            .child(
                div()
                    .mt(px(spacing::SECTION - spacing::CONTROL_GAP))
                    .flex_shrink_0()
                    .text_xs()
                    .text_color(p.muted)
                    .child(format!("Preset: {}", self.preset.name)),
            );
        div().w(px(sidebar_width)).flex_shrink_0().h_full().relative()
            .child(panel)
            .child(self.scrollbars.sidebar.element(p.border, p.muted, p.accent))
            .into_any_element()
    }

    fn browser_panel(&mut self, window: &mut Window, sidebar_width: f32, cx: &mut Context<Self>) -> AnyElement {
        let p = Palette::new(self.ui.scheme, self.ui.dark_mode);
        let visible = self.visible_ids.len();
        let candidates = visible_capture_ids(self.state.visible_items().into_iter().map(|item| item.id), &self.bundle_owner);
        let waiting = candidates.len().saturating_sub(visible);
        let failed = candidates.iter().filter(|id| self.failed_thumbnails.contains(id) || self.state.item(**id).is_some_and(|item| matches!(item.metadata, captureport_core::MetadataState::Failed(_)))).count();
        let hidden_selected = self.state.items().filter(|item| self.state.is_selected(item.id)
            && (!self.thumbnail_paths.contains_key(&item.id)
                || !matches!(item.metadata, captureport_core::MetadataState::Ready(_)))).count();
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
            .child(div().px(px(spacing::CONTENT)).pt(px(spacing::CONTENT)).pb(px(spacing::CONTROL_GAP)).flex().flex_col().gap(px(spacing::CONTENT)).border_b_1().border_color(p.border)
                .child(div().min_w_0().flex().items_baseline().justify_between().gap(px(spacing::CONTENT))
                    .child(div().flex_1().min_w_0().text_lg().font_weight(gpui::FontWeight::SEMIBOLD).truncate().child(source_name))
                    .child(div().flex_shrink_0().text_xs().text_color(p.muted).child(format!("{visible} ready · {} loading · {failed} unavailable · {} selected{}", waiting.saturating_sub(failed), self.state.selection_summary().count,
                        if hidden_selected > 0 { format!(" ({hidden_selected} without previews; review import paths)") } else { String::new() }))))
                .child(div().flex().flex_wrap().gap(px(spacing::TIGHT))
                    .child(chip("All",self.state.filter==MediaFilter::All, p, cx.listener(|t,_,_,c|t.filter(MediaFilter::All,c))))
                    .child(chip("Photos",self.state.filter==MediaFilter::Photos, p, cx.listener(|t,_,_,c|t.filter(MediaFilter::Photos,c))))
                    .child(chip("Videos",self.state.filter==MediaFilter::Videos, p, cx.listener(|t,_,_,c|t.filter(MediaFilter::Videos,c))))
                    .child(chip("New",self.state.filter==MediaFilter::New, p, cx.listener(|t,_,_,c|t.filter(MediaFilter::New,c))))
                    .child(chip("Imported",self.state.filter==MediaFilter::Imported, p, cx.listener(|t,_,_,c|t.filter(MediaFilter::Imported,c))))
                    .child(chip("Possible",self.state.filter==MediaFilter::PossibleDuplicates, p, cx.listener(|t,_,_,c|t.filter(MediaFilter::PossibleDuplicates,c))))))
            .child(div().px(px(spacing::CONTENT)).py(px(spacing::CONTROL_GAP)).flex().flex_wrap().items_center().justify_between().gap(px(spacing::CONTROL_GAP)).border_b_1().border_color(p.border)
                .child(div().flex().flex_wrap().gap(px(spacing::CONTROL_GAP))
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
                        .child(if self.marking_imported { "Marking…" } else { "Mark as imported" })))
                .child(button(Icon::ViewOptions, if self.show_view_options { "Hide view options" } else { "View options" }, p,
                    cx.listener(|t,_,_,c| { t.show_view_options = !t.show_view_options; c.notify() }))))
            .child(if self.show_view_options {
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
                        .child(div().text_lg().font_weight(gpui::FontWeight::SEMIBOLD)
                            .child(if self.scanning { "Scanning source…" } else if waiting > failed { "Preparing previews…" } else if failed > 0 { "Previews unavailable" } else if self.state.source.is_none() { "Start with a source" } else if self.state.filter == MediaFilter::All { "No media found in this source" } else { "No media matches this filter" }))
                        .child(div().text_sm().text_color(p.muted)
                            .child(if self.scanning { "Media appears after metadata and its thumbnail are ready." } else if failed > 0 && waiting == failed { "Some files could not be previewed. Check the source or video decoder, then reopen it to retry." } else if waiting > 0 { "Metadata and thumbnails are loading." } else if self.state.source.is_none() { if self.discovered_sources.is_empty() && !self.unmounted_cards.is_empty() { "Mount the card in Sources to browse media." } else if self.discovered_sources.is_empty() { "Open a folder or connect a camera to browse media." } else { "Choose a discovered camera or card in Sources to browse media." } } else if self.state.filter == MediaFilter::All { "Try another source or check that it contains supported media." } else { "Choose All to see every capture in this source." }))
                        .child(if self.state.source.is_none() && !self.scanning {
                            div().flex().flex_wrap().justify_center().gap(px(spacing::CONTROL_GAP))
                                .child(primary_button(Icon::Folder, "Open a folder…", p,
                                    cx.listener(|t, _, w, c| t.open_folder(&OpenFolder, w, c))))
                                .child(button(Icon::Camera, if self.discovering { "Scanning devices…" } else { "Scan for cameras" }, p,
                                    cx.listener(|t, _, w, c| t.discover_sources(&DiscoverSources, w, c))))
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
                                    let base = if let Some(path)=t.thumbnail_paths.get(&id) {
                                        div().w_full().h(px(image_height)).flex().items_center().justify_center().bg(p.placeholder).overflow_hidden()
                                            .child(img(path.clone()).h(px(image_height)).max_w_full().object_fit(ObjectFit::Contain))
                                    } else {
                                        div().w_full().h(px(image_height)).bg(p.placeholder)
                                    };
                                    div().id(("media-open", id.0)).relative().w_full().cursor_pointer()
                                        .on_click(cx.listener(move|t,_,_,c|t.toggle_bundle(id,c)))
                                        .child(base)
                                        .child(div().absolute().top(px(spacing::TIGHT)).right(px(spacing::TIGHT)).px(px(spacing::TIGHT)).py(px(1.)).rounded_sm()
                                            .bg(gpui::rgba(0x00000099)).text_color(rgb(0xffffff))
                                            .flex().items_center().gap(px(spacing::TIGHT))
                                            .text_xs().font_weight(gpui::FontWeight::SEMIBOLD)
                                            .child(icons::icon_sized(badge.0, icons::ICON_SIZE_COMPACT, rgb(0xffffff)))
                                            .child(badge.1))
                                        .child(if partly_selected {
                                            div().absolute().top(px(spacing::TIGHT)).left(px(spacing::TIGHT)).px(px(spacing::CONTROL_GAP)).py(px(spacing::TIGHT)).rounded_sm()
                                                .bg(p.card).text_color(p.text).text_xs()
                                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                                .child(if selected { "Selected" } else { "Part selected" })
                                                .into_any_element()
                                        } else { div().hidden().into_any_element() })
                                        .into_any_element()
                                };
                                // Block flow keeps truncated text out of flex's intrinsic-width
                                // measurement, where GPUI can cache an ellipsis-only filename.
                                let metadata = div().w_full().h(px(spacing::MEDIA_DETAILS_HEIGHT)).px(px(spacing::CONTROL_GAP)).py(px(spacing::CONTROL_GAP)).min_w_0()
                                    .child(div().id(("media-name", id.0)).w_full().min_w_0().cursor_pointer().rounded_sm()
                                        .on_click(cx.listener(move |t, _, _, c| {
                                            c.stop_propagation();
                                            t.toggle_group_selection(id, c);
                                        }))
                                        .hover(move |style| style.bg(p.selected))
                                        .child(div().w_full().text_sm().font_weight(gpui::FontWeight::SEMIBOLD)
                                            .truncate().child(item.source_name.clone())))
                                    .child(div().mt(px(spacing::TIGHT)).text_xs().text_color(p.muted).truncate().child(
                                        if time_label.is_empty() { size_label } else { format!("{time_label} · {size_label}") }))
                                    .child(div().id(("media-status", id.0)).mt(px(spacing::TIGHT)).truncate().text_xs().font_weight(gpui::FontWeight::SEMIBOLD).cursor_pointer()
                                        .on_click(cx.listener(move|t,_,_,c|t.toggle_bundle(id,c)))
                                        .text_color(if partly_selected { p.text } else { p.muted })
                                        .child(import_status_line(item)))
                                    .child(div().id(("media-disclosure", id.0)).mt(px(spacing::TIGHT)).when(members.is_none(), |style| style.hidden()).text_xs().text_color(p.muted).cursor_pointer()
                                        .on_click(cx.listener(move |t,_,_,c| t.toggle_bundle(id,c)))
                                        .child(if members.is_some() {
                                            if expanded { "Hide files" } else { "View files" }
                                        } else { "" }));
                                cards=cards.child(div().id(("media",id.0)).flex_1().min_w_0()
                                    .overflow_hidden().rounded_sm().border_1()
                                    .border_color(if partly_selected || expanded {p.accent}else{p.canvas})
                                    .bg(if selected{p.selected}else{p.canvas})
                                    .child(picture).child(metadata));
                            }
                        } else { cards=cards.child(div().flex_1()); }
                    }
                    let mut view=div().w_full().h(px(image_height + spacing::MEDIA_ROW_CHROME + if grouped { spacing::GALLERY_ROW_CHROME } else { 0. })).flex().flex_col().px(px(spacing::CONTENT)).py(px(spacing::CONTROL_GAP));
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
                                            .flex_1().min_w_0().font_weight(gpui::FontWeight::SEMIBOLD).truncate()
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

    fn expanded_bundle_panel(&self, cx: &mut Context<Self>) -> AnyElement {
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
                    .child(div().id(("group-name", primary.0)).font_weight(gpui::FontWeight::SEMIBOLD).cursor_pointer()
                        .on_click(cx.listener(move |t, _, _, c| t.toggle_group_selection(primary, c)))
                        .child(format!(
                            "{title} · {selected_count}/{} selected",
                            members.len()
                        )))
                    .child(button(
                        Icon::Collapse, "Collapse group", p,
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
                        Icon::SelectAll, "Select all in group", p,
                        cx.listener(move |t, _, _, c| t.select_bundle_members(primary, true, c)),
                    ))
                    .child(button(
                        Icon::Clear, "Deselect all in group", p,
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
                    .child(div().text_sm().font_weight(gpui::FontWeight::SEMIBOLD).child(
                        if selected { "Selected" } else { "Select" },
                    )),
            );
        }
        div().relative().flex_shrink_0().max_h(px(240.)).min_h_0().flex().flex_col()
            .child(panel)
            .child(self.scrollbars.bundle.element(p.border, p.muted, p.accent))
            .into_any_element()
    }

    fn preview_panel(&mut self, cx: &mut Context<Self>) -> AnyElement {
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
        let skipped = plan.items.iter().filter(|item| item.status == PlanStatus::Skipped).count();
        div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .child(div().px(px(spacing::CONTENT)).py(px(spacing::CONTENT)).border_b_1().border_color(p.border)
                .child(div().flex().flex_wrap().items_center().justify_between().gap(px(spacing::CONTROL_GAP))
                    .child(div().text_lg().font_weight(gpui::FontWeight::SEMIBOLD)
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
                        .child(div().px(px(spacing::CONTENT)).py(px(spacing::CONTROL_GAP)).rounded_sm().bg(p.border).text_sm().text_color(p.muted).child("Import blocked"))
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

    fn history_panel(&mut self, cx: &mut Context<Self>) -> AnyElement {
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
        div().relative().flex_1().min_w_0().min_h_0().flex().flex_col()
            .child(panel)
            .child(self.scrollbars.history.element(p.border, p.muted, p.accent))
            .into_any_element()
    }
    fn recovery_panel(&mut self, cx: &mut Context<Self>) -> AnyElement {
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
            panel = panel.child(div().p(px(spacing::CONTENT)).bg(p.card).rounded_sm().child(format!(
                "Session {} · started {} · {} recorded copies",
                session.session.id,
                session.session.started_at,
                session.imports.len()
            )));
        }
        if !self.incomplete_sessions.is_empty() {
            panel = panel.child(button(
                Icon::AddToList, "Review remaining files", p,
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
        div().relative().flex_1().min_w_0().min_h_0().flex().flex_col()
            .child(panel)
            .child(self.scrollbars.recovery.element(p.border, p.muted, p.accent))
            .into_any_element()
    }
    fn settings_panel(&mut self, cx: &mut Context<Self>) -> AnyElement {
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
                    .text_lg()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("Settings"),
            )
            .child(div().text_sm().text_color(p.muted)
                .child("Set destinations and naming, then review the safety and media rules. Appearance and source alias save separately."))
            .child(settings_section("Destinations", p))
            .child(div().text_xs().text_color(p.muted)
                .child("Choose a starting preset to replace the fields below."))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(spacing::CONTROL_GAP))
                    .child(chip(
                        "Everyday",
                        self.preset.name == "Everyday", p,
                        cx.listener(|t, _, _, c| t.choose_preset(false, c)),
                    ))
                    .child(chip(
                        "Organized",
                        self.preset.name == "Organized", p,
                        cx.listener(|t, _, _, c| t.choose_preset(true, c)),
                    )),
            )
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
        div().flex_1().min_w_0().min_h_0().flex().flex_col()
            .child(div().relative().flex_1().min_h_0().flex().flex_col()
                .child(panel)
                .child(self.scrollbars.settings.element(p.border, p.muted, p.accent)))
            .child(div().flex_shrink_0().border_t_1().border_color(p.border)
                .px(px(spacing::CONTENT)).py(px(spacing::CONTROL_GAP))
                .flex().items_center().justify_between().gap(px(spacing::CONTENT))
                .child(div().text_xs().text_color(p.muted)
                    .child("Review the import preview before copying."))
                .child(primary_button(Icon::Confirm, "Apply settings", p,
                    cx.listener(|t, _, _, c| t.apply_settings(c)))))
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
        .px(px(spacing::CONTENT))
        .py(px(spacing::TIGHT))
        .rounded_sm()
        .text_sm()
        .bg(if active { palette.selected } else { palette.card })
        .hover(move |style| style.bg(palette.selected))
        .on_click(handler)
        .child(label)
}

fn settings_section(label: &'static str, palette: Palette) -> impl IntoElement {
    div().mt(px(spacing::CONTROL_GAP)).pb(px(spacing::TIGHT)).border_b_1().border_color(palette.border)
        .text_base().font_weight(gpui::FontWeight::SEMIBOLD).child(label)
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
        .gap(px(spacing::TIGHT))
        .child(div().text_xs().text_color(palette.muted).child(label))
        .child(themed_input(input, palette, cx))
        .into_any_element()
}

fn segment_help_label(field: &str, expanded: bool) -> &'static str {
    match (field, expanded) {
        ("Photo folder template", false) => "Photo folder segments / help",
        ("Photo folder template", true) => "Hide photo folder segments",
        ("Video folder template", false) => "Video folder segments / help",
        ("Video folder template", true) => "Hide video folder segments",
        (_, false) => "Filename segments / help",
        (_, true) => "Hide filename segments",
    }
}

fn settings_template_field(
    label: &'static str,
    input: Entity<text_input::TextInput>,
    filename: bool,
    video: bool,
    segment_target: Option<&Entity<text_input::TextInput>>,
    p: Palette,
    cx: &Context<Browser>,
) -> AnyElement {
    let expanded = segment_target.is_some_and(|target| target == &input);
    let target = input.clone();
    let example = template_help::example(&input.read(cx).value(), filename, video);
    let mut field = div()
        .flex().flex_col().gap(px(spacing::TIGHT)).min_w_0().w_full()
        .child(
            div().flex().flex_wrap().items_center().justify_between().gap(px(spacing::CONTROL_GAP))
                .child(div().text_xs().text_color(p.muted).child(label))
                .child(button(
                    Icon::TemplateSyntax,
                    segment_help_label(label, expanded),
                    p,
                    cx.listener(move |t, _, _, c| {
                        t.template_segment_target = if expanded { None } else { Some(target.clone()) };
                        c.notify();
                    }),
                )),
        )
        .child(themed_input(input.clone(), p, cx))
        .child(div().text_xs().text_color(p.muted).child(if filename {
            "Use text and {segments}. Include .{extension} to keep the file type; {original_name} keeps the whole original name. Folders are not allowed here."
        } else {
            "Use / between folders and {segments} for metadata, e.g. {year}/{date:%Y-%m-%d}. Leave blank to import directly into the destination root."
        }))
        .child(div().text_xs().text_color(if example.is_ok() { p.muted } else { p.text })
            .child(match example {
                Ok(value) if value.is_empty() => "Example (sample metadata): destination root".into(),
                Ok(value) => format!("Example (sample metadata): {value}"),
                Err(error) => format!("Check template: {error}"),
            }));
    if expanded {
        let mut reference = div().flex().flex_col().gap(px(spacing::CONTROL_GAP)).py(px(spacing::CONTENT))
            .child(div().text_sm().font_weight(gpui::FontWeight::SEMIBOLD).child("Supported segments"))
            .child(div().text_xs().text_color(p.muted)
                .child("Click a segment to insert it at the cursor. Use {sequence:04} for 0001 or {session:02} for 02 (width 1–12). Date formats use %Y, %m, %d, %H, %M and %S; for example {datetime:%Y-%m-%d_%H-%M-%S}."))
            .child(div().text_xs().text_color(p.muted)
                .child("Missing lens, dimensions, duration, GPS or timestamp metadata becomes unknown; missing camera identity stays blank. Unsafe characters in metadata become underscores. Date segments use your corrected capture time; capture_time and filesystem_time show the original timestamps. The import preview shows your actual final paths."));
        let mut previous_group = "";
        for (index, token) in captureport_ingest::TEMPLATE_TOKENS.iter().enumerate() {
            if token.group != previous_group {
                reference = reference.child(div().mt(px(spacing::CONTROL_GAP)).text_sm()
                    .font_weight(gpui::FontWeight::SEMIBOLD).child(token.group));
                previous_group = token.group;
            }
            let target = input.clone();
            let syntax = token.syntax;
            reference = reference.child(div().flex().flex_wrap().items_center().gap(px(spacing::CONTROL_GAP))
                .child(div().id((label, index)).cursor_pointer().rounded_sm()
                    .px(px(spacing::CONTROL_GAP)).py(px(spacing::TIGHT)).border_1().border_color(p.border).bg(p.card)
                    .text_sm().hover(move |style| style.bg(p.selected))
                    .on_click(cx.listener(move |_, _, window, cx| {
                        target.update(cx, |input, cx| input.insert_segment(syntax, cx));
                        window.focus(&target.read(cx).focus_handle(cx));
                        cx.notify();
                    }))
                    .child(syntax))
                .child(div().text_xs().text_color(p.muted).child(token.description)));
        }
        field = field.child(reference);
    }
    field.into_any_element()
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
        .rounded_sm()
        .overflow_hidden()
        .bg(palette.card)
        .text_color(palette.text)
        .child(input)
        .into_any_element()
}

fn sidebar_nav(
    icon: crate::icons::Icon,
    label: &'static str,
    active: bool,
    palette: Palette,
    handler: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(label)
        .min_h(px(spacing::CONTROL_HEIGHT))
        .px(px(spacing::CONTENT))
        .py(px(spacing::TIGHT))
        .rounded_sm()
        .flex().items_center().gap(px(spacing::CONTROL_GAP)).cursor_pointer()
        .text_sm()
        .font_weight(if active { gpui::FontWeight::SEMIBOLD } else { gpui::FontWeight::NORMAL })
        .text_color(if active { palette.text } else { palette.muted })
        .bg(if active { palette.selected } else { palette.panel })
        .hover(move |style| style.bg(palette.selected))
        .on_click(handler)
        .child(crate::icons::icon(icon, if active { palette.text } else { palette.muted }))
        .child(label)
}

fn primary_button(
    icon: crate::icons::Icon,
    label: &'static str,
    palette: Palette,
    handler: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(label)
        .min_h(px(spacing::CONTROL_HEIGHT))
        .px(px(spacing::CONTENT))
        .py(px(spacing::TIGHT))
        .rounded_sm()
        .flex().items_center().gap(px(spacing::CONTROL_GAP)).cursor_pointer()
        .text_sm()
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .bg(palette.primary_bg)
        .text_color(palette.primary_text)
        .hover(move |style| style.bg(palette.primary_hover))
        .on_click(handler)
        .child(crate::icons::icon(icon, palette.primary_text))
        .child(label)
}

impl Render for Browser {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Palette::new(self.ui.scheme, self.ui.dark_mode);
        let sidebar_width = if f32::from(window.bounds().size.width) < 760. { 184. } else { 232. };
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
            .font_family("Adwaita Sans")
            .text_size(px(14.))
            .text_color(p.text)
            .track_focus(&self.focus_handle(cx))
            .on_action(cx.listener(Self::open_folder))
            .on_action(cx.listener(Self::open_demo))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::select_new))
            .on_action(cx.listener(Self::select_none))
            .on_action(cx.listener(Self::mark_selected_imported))
            .on_action(cx.listener(Self::import_selected))
            .on_action(cx.listener(Self::cancel_import))
            .on_action(cx.listener(Self::history))
            .on_action(cx.listener(Self::reconcile))
            .on_action(cx.listener(Self::clock))
            .on_action(cx.listener(Self::cancel_reconcile))
            .on_action(cx.listener(Self::discover_sources))
            .child(
                div()
                    .h(px(48.)).flex_shrink_0().border_b_1().border_color(p.border)
                    .px(px(spacing::CONTENT))
                    .flex()
                    .items_center()
                    .justify_between()
                    .bg(p.header)
                    .text_color(p.header_text)
                    .child(
                        div()
                            .flex().items_baseline().gap(px(spacing::CONTENT))
                            .child(div().text_base().font_weight(gpui::FontWeight::SEMIBOLD).child("CapturePort"))
                            .child(div().text_xs().text_color(p.header_muted).child("Photo and video ingest")),
                    )
                    .child(button(
                        if self.ui.dark_mode { Icon::LightMode } else { Icon::DarkMode },
                        if self.ui.dark_mode { "Light mode" } else { "Dark mode" }, p,
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
                    .min_h(px(52.)).flex_shrink_0()
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
                    .child(div().flex().items_center().gap(px(spacing::CONTENT))
                        .child(format!(
                            "{} items · {} selected · {}",
                            self.state.len(),
                            summary.count,
                            format_size(summary.bytes)
                        ))
                        .child(if self.page == Page::Browser && summary.count > 0 && !self.importing && !self.planning && self.last_import_result.is_none() {
                            primary_button(Icon::Review, "Preview import", p,
                                cx.listener(|t, _, w, c| {
                                    if t.plan.is_some() {
                                        t.page = Page::Preview;
                                        c.notify();
                                    } else {
                                        t.import_selected(&ImportSelected, w, c);
                                    }
                                }))
                                .into_any_element()
                        } else { div().hidden().into_any_element() })),
            )
    }
}

#[cfg(test)]
mod palette_tests {
    use super::*;

    fn luminance(color: Rgba) -> f32 {
        let channel = |value: f32| {
            if value <= 0.03928 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(color.r) + 0.7152 * channel(color.g) + 0.0722 * channel(color.b)
    }

    fn contrast(first: Rgba, second: Rgba) -> f32 {
        let (a, b) = (luminance(first), luminance(second));
        let (high, low) = if a > b { (a, b) } else { (b, a) };
        (high + 0.05) / (low + 0.05)
    }

    fn differs(first: Rgba, second: Rgba) -> bool {
        (first.r - second.r).abs() > 0.001
            || (first.g - second.g).abs() > 0.001
            || (first.b - second.b).abs() > 0.001
    }

    fn every_palette() -> Vec<(ColorScheme, bool, Palette)> {
        ColorScheme::ALL
            .into_iter()
            .flat_map(|scheme| {
                [false, true].map(|dark| (scheme, dark, Palette::new(scheme, dark)))
            })
            .collect()
    }

    #[test]
    fn text_stays_legible_in_every_scheme_and_mode() {
        for (scheme, dark, p) in every_palette() {
            let context = format!("{scheme:?} dark={dark}");
            for (surface, background) in [
                ("canvas", p.canvas),
                ("panel", p.panel),
                ("card", p.card),
                ("selected", p.selected),
                ("danger", p.danger),
            ] {
                let ratio = contrast(p.text, background);
                assert!(ratio >= 4.5, "{context}: text on {surface} is {ratio:.2}:1");
            }
            for (surface, background) in [("canvas", p.canvas), ("panel", p.panel), ("card", p.card)] {
                let ratio = contrast(p.muted, background);
                assert!(ratio >= 3.0, "{context}: muted on {surface} is {ratio:.2}:1");
            }
            let ratio = contrast(p.primary_text, p.primary_bg);
            assert!(ratio >= 4.5, "{context}: primary button label is {ratio:.2}:1");
            let ratio = contrast(p.header_text, p.header);
            assert!(ratio >= 4.5, "{context}: wordmark on header is {ratio:.2}:1");
            let ratio = contrast(p.header_muted, p.header);
            assert!(ratio >= 3.0, "{context}: header subtitle is {ratio:.2}:1");
            let ratio = contrast(p.border, p.canvas);
            assert!(ratio >= 1.2, "{context}: border on canvas is invisible ({ratio:.2}:1)");
        }
    }

    #[test]
    fn every_scheme_and_mode_looks_different() {
        let palettes = every_palette();
        for (index, (scheme, dark, palette)) in palettes.iter().enumerate() {
            for (other_scheme, other_dark, other) in &palettes[index + 1..] {
                let same = !differs(palette.canvas, other.canvas)
                    && !differs(palette.accent, other.accent);
                assert!(
                    !same,
                    "{scheme:?}/dark={dark} and {other_scheme:?}/dark={other_dark} render identically"
                );
            }
        }
    }

    /// The header, primary and ghost treatments used to be hardcoded greens shared by
    /// every mode, so a second scheme would have inherited the Pine chrome.
    #[test]
    fn chrome_follows_the_scheme() {
        for dark in [false, true] {
            let pine = Palette::new(ColorScheme::Pine, dark);
            for scheme in ColorScheme::ALL {
                if scheme == ColorScheme::Pine {
                    continue;
                }
                let p = Palette::new(scheme, dark);
                assert!(differs(p.header, pine.header), "{scheme:?}/dark={dark}: header");
                assert!(differs(p.primary_bg, pine.primary_bg), "{scheme:?}/dark={dark}: primary");
                assert!(differs(p.ghost_border, pine.ghost_border), "{scheme:?}/dark={dark}: ghost");
            }
        }
    }

    #[test]
    fn preferences_keep_legacy_files_and_round_trip_the_scheme() {
        let legacy: UiPreferences =
            serde_json::from_str(r#"{"dark_mode":true,"thumbnail_size":2}"#).expect("legacy ui.json");
        assert!(legacy.dark_mode);
        assert_eq!(legacy.scheme, ColorScheme::Pine);

        let prefs = UiPreferences {
            dark_mode: false,
            scheme: ColorScheme::Darkroom,
            thumbnail_size: 3,
        };
        let encoded = serde_json::to_string(&prefs).expect("encode preferences");
        assert!(encoded.contains("\"darkroom\""), "{encoded}");
        let decoded: UiPreferences = serde_json::from_str(&encoded).expect("decode preferences");
        assert_eq!(decoded.scheme, ColorScheme::Darkroom);
        assert_eq!(decoded.thumbnail_size, 3);
    }
}

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

/// A source's label. `secondary` is the detected removable mount's actual mount
/// directory, which is too long to share a line with the model in the rail.
struct SourceLabel {
    primary: String,
    secondary: Option<String>,
}

impl SourceLabel {
    fn joined(&self) -> String {
        match &self.secondary {
            Some(secondary) => format!("{} · {secondary}", self.primary),
            None => self.primary.clone(),
        }
    }
}

/// Label for a detected removable mount: the device model on the first line and
/// the actual mount directory on the second. A `Card · Disk` basename label is
/// not enough to tell one inserted volume from another. When the block device
/// reports no model, the mount directory stands alone.
fn removable_source_label(model: Option<&str>, path: &std::path::Path) -> SourceLabel {
    let directory = path.display().to_string();
    match model {
        Some(model) => SourceLabel {
            primary: model.to_string(),
            secondary: Some(directory),
        },
        None => SourceLabel {
            primary: directory,
            secondary: None,
        },
    }
}

/// The display voice: Spectral, bundled at
/// `crates/captureport/assets/fonts/Spectral-{SemiBold,Bold}.ttf` with its OFL
/// license. It carries the wordmark and every page, panel, section, and
/// empty-state heading. Dense UI — controls, navigation, filenames, metadata,
/// and labels — stays Outfit.
const DISPLAY_FONT: &str = "Spectral";

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

/// The surface and ink shared by every overlay drawn on a photograph.
///
/// An overlay sits on an image of unknown luminance, so it must carry its own
/// surface instead of borrowing a theme one. A 60% black scrim keeps white ink
/// at 5.7:1 even over a blown white frame; the `card`-on-photograph treatment
/// this replaces measured 1.19:1 over a bright frame. DESIGN.md records the rule
/// as "image badges use a solid black scrim".
fn image_badge_surface() -> (Rgba, Rgba) {
    (gpui::rgba(0x00000099), rgb(0xffffff))
}

/// Ink for the border-filled status chip.
///
/// Deliberately `text`, not `muted`: muted ink on a border fill measures 4.27:1
/// in Ink/light, under the floor for the chip's 14px label. Kept as its own
/// function so `palette_tests::status_chip_ink_clears_the_text_floor` exercises
/// the exact value the chip renders rather than a lookalike palette pair.
fn status_chip_ink(palette: Palette) -> Rgba {
    palette.text
}

/// The one status chip filled with the border token ("Import blocked" on the
/// import preview).
fn status_chip(label: &'static str, palette: Palette) -> impl IntoElement {
    div()
        .px(px(spacing::CONTENT))
        .py(px(spacing::CONTROL_GAP))
        .rounded_sm()
        .bg(palette.border)
        .text_sm()
        .text_color(status_chip_ink(palette))
        .child(label)
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
    use super::{
        media_type_badge, removable_source_label, segment_help_label, thumbnail_columns,
        thumbnail_layout,
    };
    use crate::icons::Icon;
    use std::path::Path;

    #[test]
    fn removable_source_labels_split_model_and_mount_dir() {
        let mount = Path::new("/run/media/user/Disk");
        let labelled = removable_source_label(Some("Sony A7C II"), mount);
        assert_eq!(labelled.primary, "Sony A7C II");
        assert_eq!(labelled.secondary.as_deref(), Some("/run/media/user/Disk"));
        assert_eq!(labelled.joined(), "Sony A7C II · /run/media/user/Disk");
        let unlabelled = removable_source_label(None, mount);
        assert_eq!(unlabelled.primary, "/run/media/user/Disk");
        assert_eq!(unlabelled.secondary, None);
    }

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
        .font_family(DISPLAY_FONT).text_base().font_weight(gpui::FontWeight::SEMIBOLD).child(label)
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
            // 12-14px helper, label and metadata copy is normal-size text, so its
            // WCAG AA floor is 4.5:1 — not the 3.0 reserved for large text.
            for (surface, background) in [
                ("canvas", p.canvas),
                ("panel", p.panel),
                ("card", p.card),
                ("selected", p.selected),
            ] {
                let ratio = contrast(p.muted, background);
                assert!(
                    ratio >= 4.5,
                    "{context}: muted on {surface} is {ratio:.2}:1 (needs 4.5 for 12px text)"
                );
            }
            let ratio = contrast(p.primary_text, p.primary_bg);
            assert!(ratio >= 4.5, "{context}: primary button label is {ratio:.2}:1");
            let ratio = contrast(p.header_text, p.header);
            assert!(ratio >= 4.5, "{context}: wordmark on header is {ratio:.2}:1");
            let ratio = contrast(p.header_muted, p.header);
            assert!(
                ratio >= 4.5,
                "{context}: header subtitle is {ratio:.2}:1 (it is 12px text)"
            );
            // Region separators, not control boundaries: the border-before-background
            // rule keeps these deliberately quiet. The separate control-boundary
            // case — a button, chip or field whose outline is the only marker of its
            // own extent, measuring 1.44-1.56:1 — is a recorded gap, not pinned here.
            // Raising the boundary role to 3:1 collides with the accent focus ring
            // (1.25:1 apart in Darkroom/light), so it needs the focus-indicator
            // system retuned in the same change. See docs/spec.md.
            // The "Import blocked" status chip is the one surface in the
            // interface filled with the border token. Its ink is exercised by
            // `status_chip_ink_clears_the_text_floor` below, which reads the value
            // the chip renders.
            let ratio = contrast(p.border, p.canvas);
            assert!(ratio >= 1.2, "{context}: border on canvas is invisible ({ratio:.2}:1)");
        }
    }

    /// Exercises the value the blocked-import chip actually renders: ink on its
    /// border fill. Muted ink on that fill measures 4.27:1 in Ink/light, so the
    /// chip's own ink choice is asserted here rather than a lookalike pair.
    #[test]
    fn status_chip_ink_clears_the_text_floor() {
        for (scheme, dark, p) in every_palette() {
            let ratio = contrast(status_chip_ink(p), p.border);
            assert!(
                ratio >= 4.5,
                "{scheme:?} dark={dark}: status chip ink on its border fill is {ratio:.2}:1"
            );
        }
    }

    /// Every overlay drawn on a photograph must stay readable over any frame,
    /// including a blown-out white one. The selection label borrowed the `card`
    /// surface over the photo and measured 1.19:1 on a bright frame, while the
    /// type badge already used the scrim. Both now share `image_badge_surface`.
    #[test]
    fn image_badges_stay_legible_over_any_photograph() {
        let (scrim, ink) = image_badge_surface();
        let over = |photo: f32| -> Rgba {
            let blend = |front: f32, back: f32| front * scrim.a + back * (1.0 - scrim.a);
            Rgba {
                r: blend(scrim.r, photo),
                g: blend(scrim.g, photo),
                b: blend(scrim.b, photo),
                a: 1.0,
            }
        };
        for (label, photo) in [("white", 1.0), ("mid grey", 0.5), ("black", 0.0)] {
            let ratio = contrast(ink, over(photo));
            assert!(ratio >= 4.5, "badge ink on a {label} photograph is {ratio:.2}:1");
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

    /// The unselected tile's caption sits on its own surface. If a palette
    /// retune collapses `card` onto `canvas`, the title and metadata lose the
    /// backdrop that separates them from the grid.
    #[test]
    fn unselected_tile_caption_has_its_own_surface() {
        for (scheme, dark, p) in every_palette() {
            assert!(
                differs(p.card, p.canvas),
                "{scheme:?} dark={dark}: unselected caption surface equals the page canvas"
            );
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
        // A file written before the setting existed merges RAW+JPEG pairs.
        assert!(legacy.merge_raw_jpeg);

        let prefs = UiPreferences {
            dark_mode: false,
            scheme: ColorScheme::Darkroom,
            thumbnail_size: 3,
            merge_raw_jpeg: false,
        };
        let encoded = serde_json::to_string(&prefs).expect("encode preferences");
        assert!(encoded.contains("\"darkroom\""), "{encoded}");
        let decoded: UiPreferences = serde_json::from_str(&encoded).expect("decode preferences");
        assert_eq!(decoded.scheme, ColorScheme::Darkroom);
        assert_eq!(decoded.thumbnail_size, 3);
        assert!(!decoded.merge_raw_jpeg);
    }
}

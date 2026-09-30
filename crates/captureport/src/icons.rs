//! The bundled Material Symbols Outlined icon set.
//!
//! Every icon is Google's Material Symbols Outlined (Material 3) 24px asset,
//! vendored unmodified from `google/material-design-icons` — see
//! `assets/icons/README.md` for the upstream revision and license. Files keep
//! the upstream 960-unit grid and single filled path, so the geometry stays
//! verifiable against upstream.
//!
//! GPUI paints an SVG as a mask and tints it with the element's text color, so
//! one file serves every scheme and both modes, and the family stays coherent
//! by construction: same grid, same optical weight, same corner language.
//!
//! Icons mark actions and destinations. Filter state stays text-only so the
//! quiet filter treatment in DESIGN.md is preserved.

use gpui::{AssetSource, IntoElement, Result, SharedString, div, prelude::*, px, rgb, svg};
use std::borrow::Cow;

/// The grid Material Symbols are authored on. Asserted by the asset contract
/// test so a future swap of grid size cannot pass unnoticed.
#[cfg(test)]
pub const ICON_GRID: &str = "0 -960 960 960";
/// Rendered size for a control icon: 16px beside 14px label text.
pub const ICON_SIZE: f32 = 16.;
/// Rendered size for an icon inside a 12px label, such as a tile badge.
pub const ICON_SIZE_COMPACT: f32 = 12.;

/// Interface semantics mapped onto Material Symbols names.
///
/// Variants are named after the interface meaning, not the glyph, so a glyph
/// can change without renaming every call site.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Icon {
    /// Back one level.
    Back,
    /// Insert template syntax.
    TemplateSyntax,
    /// A camera is a source.
    Camera,
    /// A completed or affirmative step.
    Confirm,
    /// Collapse an expanded section.
    Collapse,
    /// Cancel an in-flight operation.
    Cancel,
    /// Review a plan or summary.
    Review,
    /// A time or offset value.
    Time,
    /// Bring files into the library.
    Import,
    /// A generic file.
    File,
    /// Choose a folder.
    Folder,
    /// Removable storage.
    Storage,
    /// Past sessions.
    History,
    /// A still image.
    Photo,
    /// Stacked or grouped members.
    Bundle,
    /// The library catalog.
    Catalog,
    /// Add items to a list.
    AddToList,
    /// Switch to dark appearance.
    DarkMode,
    /// Rename or edit a value.
    Rename,
    /// Start a demo or playback.
    Play,
    /// Re-read devices or state.
    Refresh,
    /// Revert or keep the current state.
    Revert,
    /// Application settings.
    Settings,
    /// Adjust how a list is shown.
    ViewOptions,
    /// Select every item.
    SelectAll,
    /// Select only newly discovered items.
    SelectNew,
    /// Clear or deselect.
    Clear,
    /// Switch to light appearance.
    LightMode,
    /// Delete is destructive and irreversible.
    Delete,
    /// A video.
    Video,
}

impl Icon {
    /// The upstream Material Symbols file stem.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Back => "arrow_back",
            Self::TemplateSyntax => "data_object",
            Self::Camera => "photo_camera",
            Self::Confirm => "check",
            Self::Collapse => "expand_less",
            Self::Cancel => "cancel",
            Self::Review => "fact_check",
            Self::Time => "schedule",
            Self::Import => "download",
            Self::File => "description",
            Self::Folder => "folder_open",
            Self::Storage => "hard_drive",
            Self::History => "history",
            Self::Photo => "image",
            Self::Bundle => "layers",
            Self::Catalog => "database",
            Self::AddToList => "playlist_add",
            Self::DarkMode => "dark_mode",
            Self::Rename => "edit",
            Self::Play => "play_arrow",
            Self::Refresh => "refresh",
            Self::Revert => "undo",
            Self::Settings => "settings",
            Self::ViewOptions => "tune",
            Self::SelectAll => "check_box",
            Self::SelectNew => "add_box",
            Self::Clear => "disabled_by_default",
            Self::LightMode => "light_mode",
            Self::Delete => "delete",
            Self::Video => "videocam",
        }
    }

    pub fn path(self) -> SharedString {
        SharedString::from(format!("icons/{}.svg", self.name()))
    }
}

/// Every vendored icon file. `include_bytes!` makes a missing asset a build
/// error rather than a silently blank control at runtime.
const FILES: &[(&str, &[u8])] = &[
    (
        "icons/add_box.svg",
        include_bytes!("../assets/icons/add_box.svg"),
    ),
    (
        "icons/arrow_back.svg",
        include_bytes!("../assets/icons/arrow_back.svg"),
    ),
    (
        "icons/cancel.svg",
        include_bytes!("../assets/icons/cancel.svg"),
    ),
    (
        "icons/check.svg",
        include_bytes!("../assets/icons/check.svg"),
    ),
    (
        "icons/check_box.svg",
        include_bytes!("../assets/icons/check_box.svg"),
    ),
    (
        "icons/dark_mode.svg",
        include_bytes!("../assets/icons/dark_mode.svg"),
    ),
    (
        "icons/database.svg",
        include_bytes!("../assets/icons/database.svg"),
    ),
    (
        "icons/data_object.svg",
        include_bytes!("../assets/icons/data_object.svg"),
    ),
    (
        "icons/delete.svg",
        include_bytes!("../assets/icons/delete.svg"),
    ),
    (
        "icons/description.svg",
        include_bytes!("../assets/icons/description.svg"),
    ),
    (
        "icons/disabled_by_default.svg",
        include_bytes!("../assets/icons/disabled_by_default.svg"),
    ),
    (
        "icons/download.svg",
        include_bytes!("../assets/icons/download.svg"),
    ),
    ("icons/edit.svg", include_bytes!("../assets/icons/edit.svg")),
    (
        "icons/expand_less.svg",
        include_bytes!("../assets/icons/expand_less.svg"),
    ),
    (
        "icons/fact_check.svg",
        include_bytes!("../assets/icons/fact_check.svg"),
    ),
    (
        "icons/folder_open.svg",
        include_bytes!("../assets/icons/folder_open.svg"),
    ),
    (
        "icons/hard_drive.svg",
        include_bytes!("../assets/icons/hard_drive.svg"),
    ),
    (
        "icons/history.svg",
        include_bytes!("../assets/icons/history.svg"),
    ),
    (
        "icons/image.svg",
        include_bytes!("../assets/icons/image.svg"),
    ),
    (
        "icons/layers.svg",
        include_bytes!("../assets/icons/layers.svg"),
    ),
    (
        "icons/light_mode.svg",
        include_bytes!("../assets/icons/light_mode.svg"),
    ),
    (
        "icons/photo_camera.svg",
        include_bytes!("../assets/icons/photo_camera.svg"),
    ),
    (
        "icons/play_arrow.svg",
        include_bytes!("../assets/icons/play_arrow.svg"),
    ),
    (
        "icons/playlist_add.svg",
        include_bytes!("../assets/icons/playlist_add.svg"),
    ),
    (
        "icons/refresh.svg",
        include_bytes!("../assets/icons/refresh.svg"),
    ),
    (
        "icons/schedule.svg",
        include_bytes!("../assets/icons/schedule.svg"),
    ),
    (
        "icons/settings.svg",
        include_bytes!("../assets/icons/settings.svg"),
    ),
    ("icons/tune.svg", include_bytes!("../assets/icons/tune.svg")),
    ("icons/undo.svg", include_bytes!("../assets/icons/undo.svg")),
    (
        "icons/videocam.svg",
        include_bytes!("../assets/icons/videocam.svg"),
    ),
];

/// The two halves of the app mark, served beside the Material glyphs so `svg()`
/// can tint each one. The full-colour icon lives at `assets/captureport.svg`;
/// these masks are its geometry split by letter.
const LOGO_FILES: &[(&str, &[u8])] = &[
    ("logo/c.svg", include_bytes!("../assets/logo/c.svg")),
    ("logo/p.svg", include_bytes!("../assets/logo/p.svg")),
];

/// The app mark's palette. `assets/captureport.svg` carries the same four
/// values; `mark_palette_matches_the_icon` proves the two stay in step.
const MARK_GROUND: u32 = 0x15181a;
const MARK_RIM: u32 = 0x323a3b;
const MARK_C: u32 = 0xa6bbae;
const MARK_P: u32 = 0xc7b299;

/// Every file the asset source serves: the vendored Material glyphs plus the
/// two halves of the app mark.
fn all_files() -> impl Iterator<Item = &'static (&'static str, &'static [u8])> {
    FILES.iter().chain(LOGO_FILES.iter())
}

/// Serves the embedded Material Symbols files to GPUI's SVG renderer.
pub struct Icons;

impl AssetSource for Icons {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(all_files()
            .find(|(name, _)| *name == path)
            .map(|(_, bytes)| Cow::Borrowed(*bytes)))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(all_files()
            .filter(|(name, _)| name.starts_with(path))
            .map(|(name, _)| SharedString::from(*name))
            .collect())
    }
}

/// Draw one icon tinted to a control's text color.
pub fn icon(icon: Icon, color: impl Into<gpui::Hsla>) -> impl IntoElement {
    icon_sized(icon, ICON_SIZE, color)
}

/// Draw one icon at a size matching its label. Material's own ratio is a 24px
/// glyph against 16px text; the same ratio keeps a 12px badge label from being
/// outweighed by its mark.
pub fn icon_sized(icon: Icon, size: f32, color: impl Into<gpui::Hsla>) -> impl IntoElement {
    svg()
        .path(icon.path())
        .w(px(size))
        .h(px(size))
        .flex_shrink_0()
        .text_color(color)
}

/// The application mark as a badge: a quiet tile carrying the two single-colour
/// segment masks. Unlike the Material glyphs it keeps a fixed palette, because
/// the mark is the product's one authored colour; the geometry still comes from
/// small masks that `svg()` tints, so it stays crisp at any size.
///
/// `assets/captureport.svg` is the same mark for the desktop icon; the two
/// share the palette constants above.
pub fn logo_badge(size: f32) -> impl IntoElement {
    let radius = size * 28. / 128.;
    div()
        .relative()
        .w(px(size))
        .h(px(size))
        .flex_shrink_0()
        .rounded(px(radius))
        .bg(rgb(MARK_GROUND))
        .child(
            svg()
                .path("logo/c.svg")
                .absolute()
                .top_0()
                .left_0()
                .w(px(size))
                .h(px(size))
                .text_color(rgb(MARK_C)),
        )
        .child(
            svg()
                .path("logo/p.svg")
                .absolute()
                .top_0()
                .left_0()
                .w(px(size))
                .h(px(size))
                .text_color(rgb(MARK_P)),
        )
        .child(
            div()
                .absolute()
                .top_0()
                .left_0()
                .w(px(size))
                .h(px(size))
                .rounded(px(radius))
                .border_1()
                .border_color(rgb(MARK_RIM)),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(name: &str) -> &'static str {
        let (_, bytes) = FILES
            .iter()
            .find(|(file, _)| *file == name)
            .unwrap_or_else(|| panic!("{name} is not embedded"));
        std::str::from_utf8(bytes).expect("icon is utf8")
    }

    #[test]
    fn every_embedded_icon_is_a_material_symbol_on_the_upstream_grid() {
        for (name, _) in FILES {
            let text = body(name);
            assert!(text.starts_with("<svg"), "{name} is not an SVG document");
            assert!(
                text.contains(&format!("viewBox=\"{ICON_GRID}\"")),
                "{name} is off the Material Symbols grid"
            );
            assert!(text.contains("<path d=\""), "{name} has no filled path");
            // Material Symbols are filled shapes, never stroke outlines.
            assert!(!text.contains("stroke="), "{name} mixes in a stroke style");
        }
    }

    #[test]
    fn icon_variants_all_resolve_to_an_embedded_file() {
        let variants = [
            Icon::Back,
            Icon::TemplateSyntax,
            Icon::Camera,
            Icon::Confirm,
            Icon::Collapse,
            Icon::Cancel,
            Icon::Review,
            Icon::Time,
            Icon::Import,
            Icon::File,
            Icon::Folder,
            Icon::Storage,
            Icon::History,
            Icon::Photo,
            Icon::Bundle,
            Icon::Catalog,
            Icon::AddToList,
            Icon::DarkMode,
            Icon::Rename,
            Icon::Play,
            Icon::Refresh,
            Icon::Revert,
            Icon::Settings,
            Icon::ViewOptions,
            Icon::SelectAll,
            Icon::SelectNew,
            Icon::Clear,
            Icon::LightMode,
            Icon::Delete,
            Icon::Video,
        ];
        assert_eq!(variants.len(), FILES.len(), "icons.rs and FILES disagree");
        let mut names = variants.map(Icon::name).to_vec();
        names.sort_unstable();
        let count = names.len();
        names.dedup();
        assert_eq!(names.len(), count, "two variants share one glyph");
        for variant in variants {
            let path = variant.path();
            assert!(
                Icons.load(&path).unwrap().is_some(),
                "{path} does not resolve"
            );
        }
    }

    #[test]
    fn list_filters_by_prefix() {
        let icons = Icons;
        assert_eq!(icons.list("").unwrap().len(), all_files().count());
        assert!(icons.list("icons/missing").unwrap().is_empty());
    }

    #[test]
    fn the_two_mark_halves_are_embedded_beside_the_icon_set() {
        let icons = Icons;
        for path in ["logo/c.svg", "logo/p.svg"] {
            let bytes = icons
                .load(path)
                .unwrap()
                .unwrap_or_else(|| panic!("{path} must be embedded"));
            let text = std::str::from_utf8(&bytes).expect("mark half is utf8");
            assert!(
                text.contains("viewBox=\"0 0 128 128\""),
                "{path} grid moved"
            );
            assert!(text.contains("<path d=\""), "{path} has no geometry");
        }
    }

    #[test]
    fn mark_palette_matches_the_icon() {
        let icon = std::str::from_utf8(include_bytes!("../assets/captureport.svg")).unwrap();
        for (label, hex) in [
            ("ground", MARK_GROUND),
            ("rim", MARK_RIM),
            ("C", MARK_C),
            ("P", MARK_P),
        ] {
            let needle = format!("#{hex:06X}");
            assert!(
                icon.contains(&needle),
                "captureport.svg is missing the {label} {needle} that icons.rs renders"
            );
        }
        // Nine identical segments, four in the C and five in the P.
        let c = std::str::from_utf8(include_bytes!("../assets/logo/c.svg")).unwrap();
        let p = std::str::from_utf8(include_bytes!("../assets/logo/p.svg")).unwrap();
        assert_eq!(c.matches('Z').count(), 4, "C must stay four segments");
        assert_eq!(p.matches('Z').count(), 5, "P must stay five segments");
    }

    #[test]
    fn load_rejects_paths_outside_the_bundle() {
        let icons = Icons;
        assert!(icons.load("icons/delete.svg").unwrap().is_some());
        assert!(icons.load("icons/nope.svg").unwrap().is_none());
        assert!(icons.load("../assets/icons/delete.svg").unwrap().is_none());
    }
}

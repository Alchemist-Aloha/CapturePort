use crate::*;

pub(crate) fn button(
    icon: icons::Icon,
    label: impl Into<gpui::SharedString>,
    palette: Palette,
    handler: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let label = label.into();
    button_with_id(label.clone(), icon, label, palette, handler)
}

pub(crate) fn button_with_id(
    id: impl Into<gpui::ElementId>,
    icon: icons::Icon,
    label: impl Into<gpui::SharedString>,
    palette: Palette,
    handler: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let label = label.into();
    div()
        .id(id)
        .cursor_pointer()
        .flex()
        .items_center()
        .gap(px(spacing::CONTROL_GAP))
        .rounded_sm()
        .px(px(spacing::CONTENT))
        .py(px(spacing::TIGHT))
        .min_h(px(spacing::CONTROL_HEIGHT))
        .flex_shrink_0()
        .min_w_0()
        .max_w_full()
        .text_sm()
        .text_color(palette.text)
        .border_1()
        .border_color(palette.ghost_border)
        .bg(palette.ghost_bg)
        .hover(move |style| style.bg(palette.ghost_hover))
        .tab_index(0)
        .focus(move |style| style.border_color(palette.accent))
        .on_click(handler)
        .child(icons::icon(icon, palette.text))
        // The label truncates rather than overlapping the button's own border in
        // a narrow rail; a control must never clip its own outline.
        .child(div().min_w_0().truncate().child(i18n::tr(&label)))
}
/// A destructive action. Shares the button geometry so the danger surface is a
/// state of the same control, not a hand-rolled look-alike.
pub(crate) fn danger_button(
    icon: icons::Icon,
    label: impl Into<gpui::SharedString>,
    palette: Palette,
    handler: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let label = label.into();
    div()
        .id(label.clone())
        .cursor_pointer()
        .flex()
        .items_center()
        .gap(px(spacing::CONTROL_GAP))
        .rounded_sm()
        .px(px(spacing::CONTENT))
        .py(px(spacing::TIGHT))
        .min_h(px(spacing::CONTROL_HEIGHT))
        .flex_shrink_0()
        .text_sm()
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(palette.text)
        .border_1()
        .border_color(palette.accent)
        .bg(palette.danger)
        .tab_index(0)
        .focus(move |style| style.border_color(palette.accent))
        .on_click(handler)
        .child(icons::icon(icon, palette.text))
        .child(div().min_w_0().truncate().child(i18n::tr(&label)))
}
pub(crate) fn chip(
    label: impl Into<gpui::SharedString>,
    active: bool,
    palette: Palette,
    handler: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let label = label.into();
    let chip = div()
        .id(label.clone())
        .tab_index(0)
        .border_1()
        .border_color(gpui::rgba(0))
        .focus(move |style| style.border_color(palette.accent))
        .min_h(px(spacing::CONTROL_HEIGHT))
        .cursor_pointer()
        .flex()
        .items_center()
        .rounded_sm()
        .px(px(spacing::CONTROL_GAP))
        .py(px(spacing::TIGHT))
        .text_sm()
        .font_weight(if active {
            gpui::FontWeight::SEMIBOLD
        } else {
            gpui::FontWeight::NORMAL
        })
        .bg(if active {
            palette.selected
        } else {
            gpui::rgba(0)
        })
        .hover(move |style| {
            style.bg(if active {
                palette.selected
            } else {
                palette.ghost_hover
            })
        })
        .on_click(handler)
        .child(label);
    if active {
        chip.text_color(palette.text)
    } else {
        chip
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn button_identity_survives_theme_label_changes() {
        let palette = Palette::new(ColorScheme::Pine, false);
        let light = button_with_id(
            "theme-toggle",
            icons::Icon::LightMode,
            "Light mode",
            palette,
            |_, _, _| {},
        )
        .into_element();
        let dark = button_with_id(
            "theme-toggle",
            icons::Icon::DarkMode,
            "Dark mode",
            palette,
            |_, _, _| {},
        )
        .into_element();
        assert_eq!(gpui::Element::id(&light), Some("theme-toggle".into()));
        assert_eq!(gpui::Element::id(&light), gpui::Element::id(&dark));
        let refresh = button_with_id(
            "source-refresh",
            icons::Icon::Refresh,
            "Refresh",
            palette,
            |_, _, _| {},
        )
        .into_element();
        assert_ne!(gpui::Element::id(&light), gpui::Element::id(&refresh));
        let ordinary = button(icons::Icon::Confirm, "Save", palette, |_, _, _| {}).into_element();
        assert_eq!(gpui::Element::id(&ordinary), Some("Save".into()));
    }
}

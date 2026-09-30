use crate::*;

pub(crate) fn button(
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
        .min_w_0()
        .max_w_full()
        .text_sm()
        .text_color(palette.text)
        .border_1()
        .border_color(palette.ghost_border)
        .bg(palette.ghost_bg)
        .hover(move |style| style.bg(palette.ghost_hover))
        .on_click(handler)
        .child(icons::icon(icon, palette.text))
        // The label truncates rather than overlapping the button's own border in
        // a narrow rail; a control must never clip its own outline.
        .child(div().min_w_0().truncate().child(label))
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
        .on_click(handler)
        .child(icons::icon(icon, palette.text))
        .child(div().min_w_0().truncate().child(label))
}
pub(crate) fn chip(
    label: &'static str,
    active: bool,
    palette: Palette,
    handler: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let chip = div()
        .id(label)
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

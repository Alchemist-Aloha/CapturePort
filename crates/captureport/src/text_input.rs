//! Shared single-line text input for every settings field and gallery rename.
use crate::text_edit::{Movement, TextEdit};
use gpui::{
    App, Bounds, ClipboardItem, Context, CursorStyle, DispatchPhase, Element, ElementId,
    ElementInputHandler, Entity, EntityInputHandler, FocusHandle, Focusable, GlobalElementId,
    KeyBinding, LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, PaintQuad,
    Pixels, Point, ShapedLine, SharedString, Style, TextRun, UTF16Selection, UnderlineStyle,
    Window, actions, anchored, deferred, div, fill, point, prelude::*, relative,
};
use std::ops::Range;

actions!(
    captureport_text_input,
    [
        Backspace,
        Delete,
        DeleteWordLeft,
        DeleteWordRight,
        Left,
        Right,
        WordLeft,
        WordRight,
        Home,
        End,
        SelectLeft,
        SelectRight,
        SelectWordLeft,
        SelectWordRight,
        SelectHome,
        SelectEnd,
        SelectAll,
        Paste,
        Copy,
        Cut,
        Undo,
        Redo,
        OpenEditMenu,
    ]
);

pub(crate) fn init(cx: &mut App) {
    let context = Some("CapturePortTextInput");
    cx.bind_keys([
        KeyBinding::new("backspace", Backspace, context),
        KeyBinding::new("delete", Delete, context),
        KeyBinding::new("ctrl-backspace", DeleteWordLeft, context),
        KeyBinding::new("ctrl-delete", DeleteWordRight, context),
        KeyBinding::new("left", Left, context),
        KeyBinding::new("right", Right, context),
        KeyBinding::new("ctrl-left", WordLeft, context),
        KeyBinding::new("ctrl-right", WordRight, context),
        KeyBinding::new("home", Home, context),
        KeyBinding::new("end", End, context),
        KeyBinding::new("ctrl-home", Home, context),
        KeyBinding::new("ctrl-end", End, context),
        KeyBinding::new("shift-left", SelectLeft, context),
        KeyBinding::new("shift-right", SelectRight, context),
        KeyBinding::new("ctrl-shift-left", SelectWordLeft, context),
        KeyBinding::new("ctrl-shift-right", SelectWordRight, context),
        KeyBinding::new("shift-home", SelectHome, context),
        KeyBinding::new("shift-end", SelectEnd, context),
        KeyBinding::new("ctrl-shift-home", SelectHome, context),
        KeyBinding::new("ctrl-shift-end", SelectEnd, context),
        KeyBinding::new("ctrl-a", SelectAll, context),
        KeyBinding::new("ctrl-v", Paste, context),
        KeyBinding::new("shift-insert", Paste, context),
        KeyBinding::new("ctrl-c", Copy, context),
        KeyBinding::new("ctrl-insert", Copy, context),
        KeyBinding::new("ctrl-x", Cut, context),
        KeyBinding::new("shift-delete", Cut, context),
        KeyBinding::new("ctrl-z", Undo, context),
        KeyBinding::new("ctrl-shift-z", Redo, context),
        KeyBinding::new("ctrl-y", Redo, context),
        KeyBinding::new("shift-f10", OpenEditMenu, context),
        KeyBinding::new("menu", OpenEditMenu, context),
    ]);
}

#[derive(Clone, Copy)]
enum EditCommand {
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,
}
const COMMANDS: [(EditCommand, &str, &str); 6] = [
    (EditCommand::Undo, "Undo", "Ctrl+Z"),
    (EditCommand::Redo, "Redo", "Ctrl+Shift+Z"),
    (EditCommand::Cut, "Cut", "Ctrl+X"),
    (EditCommand::Copy, "Copy", "Ctrl+C"),
    (EditCommand::Paste, "Paste", "Ctrl+V"),
    (EditCommand::SelectAll, "Select all", "Ctrl+A"),
];
struct EditMenu {
    position: Point<Pixels>,
    selected: usize,
}

pub struct TextInput {
    focus_handle: FocusHandle,
    edit: TextEdit,
    placeholder: SharedString,
    selecting: bool,
    drag_position: Option<Point<Pixels>>,
    menu: Option<EditMenu>,
    palette: crate::Palette,
    last_layout: Option<ShapedLine>,
    last_bounds: Option<Bounds<Pixels>>,
    scroll_x: Pixels,
}

impl TextInput {
    pub fn new(
        initial: impl Into<SharedString>,
        placeholder: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        let initial = initial.into();
        Self {
            focus_handle: cx.focus_handle().tab_stop(true),
            edit: TextEdit::new(&initial),
            placeholder: placeholder.into(),
            selecting: false,
            drag_position: None,
            menu: None,
            palette: crate::Palette::new(crate::ColorScheme::default(), false),
            last_layout: None,
            last_bounds: None,
            scroll_x: gpui::px(0.),
        }
    }
    pub fn value(&self) -> String {
        self.edit.content.clone()
    }
    pub fn insert_segment(&mut self, segment: &str, cx: &mut Context<Self>) {
        self.edit.replace_selection(segment);
        self.menu = None;
        cx.notify();
    }
    pub fn set_value(&mut self, value: impl Into<SharedString>, cx: &mut Context<Self>) {
        let value = value.into();
        self.edit = TextEdit::new(&value);
        self.selecting = false;
        self.menu = None;
        self.scroll_x = gpui::px(0.);
        cx.notify();
    }
    pub(crate) fn set_palette(&mut self, palette: crate::Palette) {
        self.palette = palette;
    }
    fn index_for_mouse(&self, position: Point<Pixels>) -> usize {
        match (&self.last_bounds, &self.last_layout) {
            (Some(bounds), Some(line)) if !self.edit.content.is_empty() => {
                line.closest_index_for_x(position.x - bounds.left() + self.scroll_x)
            }
            _ => 0,
        }
    }
    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle);
        self.menu = None;
        self.selecting = true;
        self.drag_position = Some(event.position);
        let at = self.index_for_mouse(event.position);
        if event.modifiers.shift {
            self.edit.move_to(at, true);
        } else if event.click_count >= 3 {
            self.edit.select_all();
        } else if event.click_count == 2 {
            self.edit.select_word(at);
        } else {
            self.edit.move_to(at, false);
        }
        cx.stop_propagation();
        cx.notify();
    }
    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if event.pressed_button != Some(MouseButton::Left) {
            self.selecting = false;
            return;
        }
        self.drag_position = Some(event.position);
        self.edit
            .move_to(self.index_for_mouse(event.position), true);
        cx.notify();
    }
    fn on_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, _: &mut Context<Self>) {
        self.selecting = false;
        self.drag_position = None;
    }
    fn move_cursor(&mut self, movement: Movement, extend: bool, cx: &mut Context<Self>) {
        self.menu = None;
        self.edit.move_cursor(movement, extend);
        cx.notify();
    }
    fn delete(&mut self, movement: Movement, cx: &mut Context<Self>) {
        self.menu = None;
        self.edit.delete(movement);
        cx.notify();
    }
    fn enabled(&self, command: EditCommand, cx: &App) -> bool {
        match command {
            EditCommand::Undo => self.edit.can_undo(),
            EditCommand::Redo => self.edit.can_redo(),
            EditCommand::Cut | EditCommand::Copy => !self.edit.selection.is_empty(),
            EditCommand::Paste => cx
                .read_from_clipboard()
                .and_then(|item| item.text())
                .is_some_and(|text| !text.is_empty()),
            EditCommand::SelectAll => !self.edit.content.is_empty(),
        }
    }
    fn command(&mut self, command: EditCommand, window: &mut Window, cx: &mut Context<Self>) {
        self.menu = None;
        match command {
            EditCommand::Copy | EditCommand::Cut if !self.edit.selection.is_empty() => {
                cx.write_to_clipboard(ClipboardItem::new_string(
                    self.edit.selected_text().to_owned(),
                ));
                if matches!(command, EditCommand::Cut) {
                    self.edit.replace_selection("");
                }
            }
            EditCommand::Paste => {
                if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text())
                    && !text.is_empty()
                {
                    self.edit.replace_selection(&text);
                }
            }
            EditCommand::SelectAll => self.edit.select_all(),
            EditCommand::Undo => self.edit.undo(),
            EditCommand::Redo => self.edit.redo(),
            _ => {}
        }
        window.focus(&self.focus_handle);
        cx.notify();
    }
    fn open_menu(&mut self, position: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        self.selecting = false;
        window.focus(&self.focus_handle);
        let selected = COMMANDS
            .iter()
            .position(|(command, _, _)| self.enabled(*command, cx))
            .unwrap_or(0);
        self.menu = Some(EditMenu { position, selected });
        cx.notify();
    }
    fn edit_menu(&self, cx: &Context<Self>) -> Option<gpui::AnyElement> {
        let menu = self.menu.as_ref()?;
        let p = self.palette;
        let mut view = div()
            .id("text-edit-menu")
            .w(gpui::px(260.))
            .py(gpui::px(crate::spacing::TIGHT))
            .border_1()
            .border_color(p.border)
            .rounded_sm()
            .bg(p.card)
            .text_color(p.text)
            .flex()
            .flex_col()
            .on_mouse_down_out(cx.listener(|t, _, _, cx| {
                t.menu = None;
                cx.notify();
            }));
        for (index, &(command, label, shortcut)) in COMMANDS.iter().enumerate() {
            let enabled = self.enabled(command, cx);
            view = view.child(
                div()
                    .id(index)
                    .px(gpui::px(crate::spacing::CONTENT))
                    .min_h(gpui::px(crate::spacing::CONTROL_HEIGHT))
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_sm()
                    .text_color(if enabled { p.text } else { p.muted })
                    .when(index == menu.selected && enabled, |v| v.bg(p.selected))
                    .when(index == 2 || index == 5, |v| {
                        v.border_t_1().border_color(p.border)
                    })
                    .when(enabled, |v| {
                        v.cursor_pointer()
                            .hover(move |v| v.bg(p.selected))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |t, _, window, cx| {
                                    cx.stop_propagation();
                                    t.command(command, window, cx);
                                }),
                            )
                    })
                    .child(crate::i18n::tr(label))
                    .child(div().text_xs().text_color(p.muted).child(shortcut)),
            );
        }
        Some(
            deferred(
                anchored()
                    .position(menu.position)
                    .snap_to_window()
                    .child(view),
            )
            .with_priority(1)
            .into_any_element(),
        )
    }
    fn menu_key(
        &mut self,
        event: &gpui::KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(menu) = &self.menu else {
            return;
        };
        let selected = menu.selected;
        match event.keystroke.key.as_str() {
            "escape" => {
                self.menu = None;
            }
            "enter" => {
                if self.enabled(COMMANDS[selected].0, cx) {
                    self.command(COMMANDS[selected].0, window, cx);
                }
            }
            "up" | "down" => {
                let step = if event.keystroke.key == "up" {
                    COMMANDS.len() - 1
                } else {
                    1
                };
                for i in 1..=COMMANDS.len() {
                    let next = (selected + step * i) % COMMANDS.len();
                    if self.enabled(COMMANDS[next].0, cx) {
                        self.menu.as_mut().unwrap().selected = next;
                        break;
                    }
                }
            }
            "tab" => {
                self.menu = None;
                cx.notify();
                return;
            }
            _ => return,
        }
        cx.stop_propagation();
        cx.notify();
    }
}

impl EntityInputHandler for TextInput {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        actual: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.edit.range_from_utf16(range);
        *actual = Some(self.edit.utf8_to_utf16(range.start)..self.edit.utf8_to_utf16(range.end));
        Some(self.edit.content[range].to_owned())
    }
    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.edit.utf8_to_utf16(self.edit.selection.start)
                ..self.edit.utf8_to_utf16(self.edit.selection.end),
            reversed: self.edit.reversed,
        })
    }
    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.edit
            .marked_range
            .as_ref()
            .map(|r| self.edit.utf8_to_utf16(r.start)..self.edit.utf8_to_utf16(r.end))
    }
    fn unmark_text(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.edit.marked_range = None;
        cx.notify();
    }
    fn replace_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.edit.replace_utf16(range, text);
        self.menu = None;
        cx.notify();
    }
    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        selected: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.edit.compose_utf16(range, text, selected);
        self.menu = None;
        cx.notify();
    }
    fn bounds_for_range(
        &mut self,
        range: Range<usize>,
        _: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let bounds = self.last_bounds?;
        let line = self.last_layout.as_ref()?;
        let range = self.edit.range_from_utf16(range);
        Some(Bounds::from_corners(
            point(
                bounds.left() - self.scroll_x + line.x_for_index(range.start),
                bounds.top(),
            ),
            point(
                bounds.left() - self.scroll_x + line.x_for_index(range.end),
                bounds.bottom(),
            ),
        ))
    }
    fn character_index_for_point(
        &mut self,
        position: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        self.last_layout.as_ref()?;
        self.last_bounds?;
        Some(self.edit.utf8_to_utf16(self.index_for_mouse(position)))
    }
}
impl Focusable for TextInput {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

struct TextElement {
    input: Entity<TextInput>,
}
struct PaintState {
    line: Option<ShapedLine>,
    cursor: Option<PaintQuad>,
    selection: Option<PaintQuad>,
    scroll_x: Pixels,
}
impl IntoElement for TextElement {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for TextElement {
    type RequestLayoutState = ();
    type PrepaintState = PaintState;
    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = window.line_height().into();
        (window.request_layout(style, [], cx), ())
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> PaintState {
        let input = self.input.read(cx);
        let content: SharedString = if input.edit.content.is_empty() {
            crate::i18n::tr(&input.placeholder).into()
        } else {
            input.edit.content.clone().into()
        };
        let color = if input.edit.content.is_empty() {
            let mut color = window.text_style().color;
            color.a *= 0.65;
            color
        } else {
            window.text_style().color
        };
        let run = TextRun {
            len: content.len(),
            font: window.text_style().font(),
            color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let runs = if let Some(marked) = &input.edit.marked_range {
            vec![
                TextRun {
                    len: marked.start,
                    ..run.clone()
                },
                TextRun {
                    len: marked.len(),
                    underline: Some(UnderlineStyle {
                        color: Some(run.color),
                        thickness: gpui::px(1.),
                        wavy: false,
                    }),
                    ..run.clone()
                },
                TextRun {
                    len: input.edit.content.len() - marked.end,
                    ..run
                },
            ]
            .into_iter()
            .filter(|run| run.len > 0)
            .collect::<Vec<_>>()
        } else {
            vec![run]
        };
        let line = window.text_system().shape_line(
            content,
            window.text_style().font_size.to_pixels(window.rem_size()),
            &runs,
            None,
        );
        let cursor_pos = line.x_for_index(input.edit.cursor());
        let mut scroll_x = input.scroll_x;
        if cursor_pos < scroll_x {
            scroll_x = cursor_pos;
        } else if cursor_pos > scroll_x + bounds.size.width - gpui::px(2.) {
            scroll_x = cursor_pos - bounds.size.width + gpui::px(2.);
        }
        scroll_x = scroll_x.max(gpui::px(0.));
        let mut selection_color = window.text_style().color;
        selection_color.a *= 0.25;
        let (selection, cursor) = if input.edit.selection.is_empty() {
            (
                None,
                Some(fill(
                    Bounds::new(
                        point(bounds.left() - scroll_x + cursor_pos, bounds.top()),
                        gpui::size(gpui::px(2.), bounds.bottom() - bounds.top()),
                    ),
                    window.text_style().color,
                )),
            )
        } else {
            (
                Some(fill(
                    Bounds::from_corners(
                        point(
                            bounds.left() - scroll_x + line.x_for_index(input.edit.selection.start),
                            bounds.top(),
                        ),
                        point(
                            bounds.left() - scroll_x + line.x_for_index(input.edit.selection.end),
                            bounds.bottom(),
                        ),
                    ),
                    selection_color,
                )),
                None,
            )
        };
        PaintState {
            line: Some(line),
            cursor,
            selection,
            scroll_x,
        }
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        state: &mut PaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus = self.input.read(cx).focus_handle.clone();
        window.handle_input(
            &focus,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );
        if let Some(selection) = state.selection.take() {
            window.paint_quad(selection);
        }
        let line = state.line.take().unwrap();
        line.paint(
            point(bounds.left() - state.scroll_x, bounds.top()),
            window.line_height(),
            window,
            cx,
        )
        .unwrap();
        if focus.is_focused(window)
            && let Some(cursor) = state.cursor.take()
        {
            window.paint_quad(cursor);
        }
        self.input.update(cx, |input, cx| {
            input.last_layout = Some(line);
            input.last_bounds = Some(bounds);
            input.scroll_x = state.scroll_x;
            if input.selecting
                && let Some(position) = input.drag_position
            {
                let movement = if position.x < bounds.left() {
                    Some(Movement::Left)
                } else if position.x > bounds.right() {
                    Some(Movement::Right)
                } else {
                    None
                };
                if let Some(movement) = movement {
                    let before = input.edit.selection.clone();
                    input.edit.move_cursor(movement, true);
                    if input.edit.selection != before {
                        cx.notify();
                        window.request_animation_frame();
                    }
                }
            }
        });
        let input = self.input.clone();
        window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
            if phase == DispatchPhase::Bubble && input.read(cx).selecting {
                input.update(cx, |input, cx| input.on_mouse_move(event, window, cx));
            }
        });
    }
}

impl Render for TextInput {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.focus_handle.is_focused(window) {
            self.menu = None;
        }
        let menu = self.edit_menu(cx);
        div()
            .id("text-input")
            .w_full()
            .min_w_0()
            .key_context("CapturePortTextInput")
            .track_focus(&self.focus_handle)
            .cursor(CursorStyle::IBeam)
            .on_action(cx.listener(|t, _: &Backspace, _, cx| t.delete(Movement::Left, cx)))
            .on_action(cx.listener(|t, _: &Delete, _, cx| t.delete(Movement::Right, cx)))
            .on_action(cx.listener(|t, _: &DeleteWordLeft, _, cx| t.delete(Movement::WordLeft, cx)))
            .on_action(
                cx.listener(|t, _: &DeleteWordRight, _, cx| t.delete(Movement::WordRight, cx)),
            )
            .on_action(cx.listener(|t, _: &Left, _, cx| t.move_cursor(Movement::Left, false, cx)))
            .on_action(cx.listener(|t, _: &Right, _, cx| t.move_cursor(Movement::Right, false, cx)))
            .on_action(
                cx.listener(|t, _: &WordLeft, _, cx| t.move_cursor(Movement::WordLeft, false, cx)),
            )
            .on_action(
                cx.listener(|t, _: &WordRight, _, cx| {
                    t.move_cursor(Movement::WordRight, false, cx)
                }),
            )
            .on_action(cx.listener(|t, _: &Home, _, cx| t.move_cursor(Movement::Start, false, cx)))
            .on_action(cx.listener(|t, _: &End, _, cx| t.move_cursor(Movement::End, false, cx)))
            .on_action(
                cx.listener(|t, _: &SelectLeft, _, cx| t.move_cursor(Movement::Left, true, cx)),
            )
            .on_action(
                cx.listener(|t, _: &SelectRight, _, cx| t.move_cursor(Movement::Right, true, cx)),
            )
            .on_action(cx.listener(|t, _: &SelectWordLeft, _, cx| {
                t.move_cursor(Movement::WordLeft, true, cx)
            }))
            .on_action(cx.listener(|t, _: &SelectWordRight, _, cx| {
                t.move_cursor(Movement::WordRight, true, cx)
            }))
            .on_action(
                cx.listener(|t, _: &SelectHome, _, cx| t.move_cursor(Movement::Start, true, cx)),
            )
            .on_action(
                cx.listener(|t, _: &SelectEnd, _, cx| t.move_cursor(Movement::End, true, cx)),
            )
            .on_action(
                cx.listener(|t, _: &SelectAll, w, cx| t.command(EditCommand::SelectAll, w, cx)),
            )
            .on_action(cx.listener(|t, _: &Paste, w, cx| t.command(EditCommand::Paste, w, cx)))
            .on_action(cx.listener(|t, _: &Copy, w, cx| t.command(EditCommand::Copy, w, cx)))
            .on_action(cx.listener(|t, _: &Cut, w, cx| t.command(EditCommand::Cut, w, cx)))
            .on_action(cx.listener(|t, _: &Undo, w, cx| t.command(EditCommand::Undo, w, cx)))
            .on_action(cx.listener(|t, _: &Redo, w, cx| t.command(EditCommand::Redo, w, cx)))
            .on_action(cx.listener(|t, _: &OpenEditMenu, w, cx| {
                let position = t
                    .last_bounds
                    .map(|bounds| point(bounds.left(), bounds.bottom()))
                    .unwrap_or_default();
                t.open_menu(position, w, cx);
            }))
            .on_key_down(cx.listener(Self::menu_key))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|t, event: &MouseDownEvent, w, cx| {
                    if t.edit.selection.is_empty() {
                        t.edit.move_to(t.index_for_mouse(event.position), false);
                    }
                    t.open_menu(event.position, w, cx);
                    cx.stop_propagation();
                }),
            )
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .min_h(gpui::px(crate::spacing::CONTROL_HEIGHT - 2.))
            .px(gpui::px(crate::spacing::CONTENT))
            .py(gpui::px(crate::spacing::TIGHT))
            .flex()
            .items_center()
            .child(TextElement { input: cx.entity() })
            .children(menu)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn edit_menu_commands_have_catalog_entries() {
        let catalog: std::collections::BTreeMap<String, String> =
            serde_json::from_str(include_str!("../assets/locales/en.json")).unwrap();
        for (_, label, _) in super::COMMANDS {
            assert!(
                catalog.contains_key(label),
                "missing edit-menu label: {label}"
            );
        }
    }
}

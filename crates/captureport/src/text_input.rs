//! Small single-line text editor used by the settings and planning surfaces.
//!
//! GPUI deliberately exposes text input through `EntityInputHandler` rather
//! than providing a ready-made widget.  Keeping that plumbing here prevents
//! each settings field from implementing a subtly different editor.

use std::ops::Range;

use gpui::{
    App, Bounds, Context, CursorStyle, Element, ElementId, ElementInputHandler, Entity,
    EntityInputHandler, FocusHandle, Focusable, GlobalElementId, LayoutId, MouseButton,
    MouseDownEvent, PaintQuad, Pixels, Point, ShapedLine, SharedString, Style, TextRun,
    UTF16Selection, Window, actions, div, fill, point, prelude::*, relative,
};

actions!(
    captureport_text_input,
    [Backspace, Delete, Left, Right, Home, End, SelectAll]
);

/// A single-line editable text value.
pub struct TextInput {
    focus_handle: FocusHandle,
    content: SharedString,
    placeholder: SharedString,
    selection: Range<usize>,
    reversed: bool,
    marked_range: Option<Range<usize>>,
    last_layout: Option<ShapedLine>,
    last_bounds: Option<Bounds<Pixels>>,
}

impl TextInput {
    pub fn new(
        initial: impl Into<SharedString>,
        placeholder: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        let content = initial.into();
        let end = content.len();
        Self {
            focus_handle: cx.focus_handle(),
            content,
            placeholder: placeholder.into(),
            selection: end..end,
            reversed: false,
            marked_range: None,
            last_layout: None,
            last_bounds: None,
        }
    }

    pub fn value(&self) -> String {
        self.content.to_string()
    }
    /// Insert a template segment at the cursor, replacing selected text.
    pub fn insert_segment(&mut self, segment: &str, cx: &mut Context<Self>) {
        let range = self.selection.clone();
        self.content =
            (self.content[..range.start].to_owned() + segment + &self.content[range.end..]).into();
        let at = range.start + segment.len();
        self.selection = at..at;
        self.reversed = false;
        self.marked_range = None;
        cx.notify();
    }
    pub fn set_value(&mut self, value: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.content = value.into();
        let end = self.content.len();
        self.selection = end..end;
        self.marked_range = None;
        cx.notify();
    }

    fn cursor(&self) -> usize {
        if self.reversed {
            self.selection.start
        } else {
            self.selection.end
        }
    }
    fn previous(&self, at: usize) -> usize {
        self.content
            .char_indices()
            .rev()
            .find_map(|(i, _)| (i < at).then_some(i))
            .unwrap_or(0)
    }
    fn next(&self, at: usize) -> usize {
        self.content
            .char_indices()
            .find_map(|(i, _)| (i > at).then_some(i))
            .unwrap_or(self.content.len())
    }
    fn move_to(&mut self, at: usize, cx: &mut Context<Self>) {
        self.selection = at..at;
        self.reversed = false;
        cx.notify();
    }
    fn replace(&mut self, text: &str, _window: &mut Window, cx: &mut Context<Self>) {
        let range = self.selection.clone();
        self.content =
            (self.content[..range.start].to_owned() + text + &self.content[range.end..]).into();
        let at = range.start + text.len();
        self.selection = at..at;
        self.marked_range = None;
        cx.notify();
    }
    fn on_mouse_down(&mut self, _: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle(cx));
    }
    fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        if self.selection.is_empty() {
            let at = self.cursor();
            self.selection = self.previous(at)..at;
        }
        self.replace("", window, cx);
    }
    fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        if self.selection.is_empty() {
            let at = self.cursor();
            self.selection = at..self.next(at);
        }
        self.replace("", window, cx);
    }
    fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(
            if self.selection.is_empty() {
                self.previous(self.cursor())
            } else {
                self.selection.start
            },
            cx,
        );
    }
    fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(
            if self.selection.is_empty() {
                self.next(self.cursor())
            } else {
                self.selection.end
            },
            cx,
        );
    }
    fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
    }
    fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.content.len(), cx);
    }
    fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.selection = 0..self.content.len();
        self.reversed = false;
        cx.notify();
    }
    fn utf8_to_utf16(&self, at: usize) -> usize {
        self.content[..at].encode_utf16().count()
    }
    fn utf16_to_utf8(&self, at: usize) -> usize {
        let mut utf16 = 0;
        for (index, ch) in self.content.char_indices() {
            if utf16 >= at {
                return index;
            }
            utf16 += ch.len_utf16();
        }
        self.content.len()
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
        let range = self.utf16_to_utf8(range.start)..self.utf16_to_utf8(range.end);
        *actual = Some(self.utf8_to_utf16(range.start)..self.utf8_to_utf16(range.end));
        Some(self.content[range].to_string())
    }
    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.utf8_to_utf16(self.selection.start)..self.utf8_to_utf16(self.selection.end),
            reversed: self.reversed,
        })
    }
    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.marked_range
            .as_ref()
            .map(|r| self.utf8_to_utf16(r.start)..self.utf8_to_utf16(r.end))
    }
    fn unmark_text(&mut self, _: &mut Window, _: &mut Context<Self>) {
        self.marked_range = None;
    }
    fn replace_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range
            .map(|r| self.utf16_to_utf8(r.start)..self.utf16_to_utf8(r.end))
            .unwrap_or_else(|| self.selection.clone());
        self.content =
            (self.content[..range.start].to_owned() + text + &self.content[range.end..]).into();
        let at = range.start + text.len();
        self.selection = at..at;
        self.marked_range = None;
        cx.notify();
    }
    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        selected: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.replace_text_in_range(range, text, window, cx);
        if !text.is_empty() {
            let at = self.cursor();
            self.marked_range = Some(at - text.len()..at);
        }
        if let Some(selected) = selected {
            let at = self.cursor();
            self.selection =
                at + self.utf16_to_utf8(selected.start)..at + self.utf16_to_utf8(selected.end);
        }
        cx.notify();
    }
    fn bounds_for_range(
        &mut self,
        range: Range<usize>,
        bounds: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let line = self.last_layout.as_ref()?;
        let range = self.utf16_to_utf8(range.start)..self.utf16_to_utf8(range.end);
        Some(Bounds::from_corners(
            point(bounds.left() + line.x_for_index(range.start), bounds.top()),
            point(bounds.left() + line.x_for_index(range.end), bounds.bottom()),
        ))
    }
    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        let bounds = self.last_bounds?;
        let line = self.last_layout.as_ref()?;
        Some(self.utf8_to_utf16(line.index_for_x(point.x - bounds.left())?))
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
        let content = if input.content.is_empty() {
            input.placeholder.clone()
        } else {
            input.content.clone()
        };
        let color = if input.content.is_empty() {
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
        let line = window.text_system().shape_line(
            content,
            window.text_style().font_size.to_pixels(window.rem_size()),
            &[run],
            None,
        );
        let cursor_pos = line.x_for_index(input.cursor());
        let mut selection_color = window.text_style().color;
        selection_color.a *= 0.25;
        let (selection, cursor) = if input.selection.is_empty() {
            (
                None,
                Some(fill(
                    Bounds::new(
                        point(bounds.left() + cursor_pos, bounds.top()),
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
                            bounds.left() + line.x_for_index(input.selection.start),
                            bounds.top(),
                        ),
                        point(
                            bounds.left() + line.x_for_index(input.selection.end),
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
        line.paint(bounds.origin, window.line_height(), window, cx)
            .unwrap();
        if focus.is_focused(window)
            && let Some(cursor) = state.cursor.take()
        {
            window.paint_quad(cursor);
        }
        self.input.update(cx, |input, _| {
            input.last_layout = Some(line);
            input.last_bounds = Some(bounds);
        });
    }
}

impl Render for TextInput {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w_full()
            .min_w_0()
            .key_context("CapturePortTextInput")
            .track_focus(&self.focus_handle(cx))
            .cursor(CursorStyle::IBeam)
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::select_all))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .min_h(gpui::px(crate::spacing::CONTROL_HEIGHT - 2.))
            .px(gpui::px(crate::spacing::CONTENT))
            .py(gpui::px(crate::spacing::TIGHT))
            .flex()
            .items_center()
            .child(TextElement { input: cx.entity() })
    }
}

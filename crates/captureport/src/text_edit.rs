//! Single-line editing state, tested without a compositor or system clipboard.
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Movement {
    Left,
    Right,
    WordLeft,
    WordRight,
    Start,
    End,
}

#[derive(Clone, Debug, PartialEq)]
struct Snapshot {
    content: String,
    selection: Range<usize>,
    reversed: bool,
}

pub(crate) struct TextEdit {
    pub(crate) content: String,
    pub(crate) selection: Range<usize>,
    pub(crate) reversed: bool,
    pub(crate) marked_range: Option<Range<usize>>,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
}

pub(crate) fn single_line_text(text: &str) -> String {
    text.replace(['\r', '\n', '\t', '\u{2028}', '\u{2029}'], " ")
}

impl TextEdit {
    pub(crate) fn new(content: &str) -> Self {
        let content = single_line_text(content);
        let end = content.len();
        Self {
            content,
            selection: end..end,
            reversed: false,
            marked_range: None,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }
    pub(crate) fn cursor(&self) -> usize {
        if self.reversed {
            self.selection.start
        } else {
            self.selection.end
        }
    }
    pub(crate) fn selected_text(&self) -> &str {
        &self.content[self.selection.clone()]
    }
    pub(crate) fn move_to(&mut self, at: usize, extend: bool) {
        self.marked_range = None;
        let at = floor_boundary(&self.content, at);
        if extend {
            let anchor = if self.reversed {
                self.selection.end
            } else {
                self.selection.start
            };
            self.selection = anchor.min(at)..anchor.max(at);
            self.reversed = at < anchor;
        } else {
            self.selection = at..at;
            self.reversed = false;
        }
    }
    pub(crate) fn select_all(&mut self) {
        self.marked_range = None;
        self.selection = 0..self.content.len();
        self.reversed = false;
    }
    pub(crate) fn move_cursor(&mut self, movement: Movement, extend: bool) {
        let at = if !extend && !self.selection.is_empty() && movement == Movement::Left {
            self.selection.start
        } else if !extend && !self.selection.is_empty() && movement == Movement::Right {
            self.selection.end
        } else {
            self.boundary(movement)
        };
        self.move_to(at, extend);
    }
    fn boundary(&self, movement: Movement) -> usize {
        let at = self.cursor();
        match movement {
            Movement::Start => 0,
            Movement::End => self.content.len(),
            Movement::Left => self
                .content
                .grapheme_indices(true)
                .rev()
                .find_map(|(i, _)| (i < at).then_some(i))
                .unwrap_or(0),
            Movement::Right => self
                .content
                .grapheme_indices(true)
                .find_map(|(i, _)| (i > at).then_some(i))
                .unwrap_or(self.content.len()),
            Movement::WordLeft => self.content[..at]
                .unicode_word_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0),
            Movement::WordRight => self.content[at..]
                .unicode_word_indices()
                .next()
                .map(|(i, word)| at + i + word.len())
                .unwrap_or(self.content.len()),
        }
    }
    pub(crate) fn select_word(&mut self, at: usize) {
        self.marked_range = None;
        let at = at.min(self.content.len());
        let at = if at == self.content.len() {
            self.content
                .grapheme_indices(true)
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0)
        } else {
            at
        };
        if let Some((start, word)) = self
            .content
            .split_word_bound_indices()
            .find(|(start, word)| (*start..start + word.len()).contains(&at))
        {
            self.selection = start..start + word.len();
            self.reversed = false;
        }
    }
    pub(crate) fn delete(&mut self, movement: Movement) {
        if self.selection.is_empty() {
            self.move_cursor(movement, true);
        }
        self.replace_selection("");
    }
    pub(crate) fn replace_selection(&mut self, text: &str) {
        self.marked_range = None;
        self.replace(self.selection.clone(), text);
    }
    pub(crate) fn utf8_to_utf16(&self, at: usize) -> usize {
        self.content[..floor_boundary(&self.content, at)]
            .encode_utf16()
            .count()
    }
    pub(crate) fn utf16_to_utf8(&self, at: usize) -> usize {
        utf16_to_utf8(&self.content, at)
    }
    pub(crate) fn range_from_utf16(&self, range: Range<usize>) -> Range<usize> {
        let start = self.utf16_to_utf8(range.start);
        let end = self.utf16_to_utf8(range.end);
        start.min(end)..start.max(end)
    }
    fn replacement_range(&self, range: Option<Range<usize>>) -> Range<usize> {
        range
            .map(|range| self.range_from_utf16(range))
            .or_else(|| self.marked_range.clone())
            .unwrap_or_else(|| self.selection.clone())
    }
    pub(crate) fn replace_utf16(&mut self, range: Option<Range<usize>>, text: &str) {
        let range = self.replacement_range(range);
        self.replace(range, text);
        self.marked_range = None;
    }
    pub(crate) fn compose_utf16(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        selected: Option<Range<usize>>,
    ) {
        let range = self.replacement_range(range);
        let start = range.start;
        let text = single_line_text(text);
        self.replace(range, &text);
        self.marked_range = (!text.is_empty()).then_some(start..start + text.len());
        if let Some(selected) = selected {
            let a = start + utf16_to_utf8(&text, selected.start);
            let b = start + utf16_to_utf8(&text, selected.end);
            self.selection = a.min(b)..a.max(b);
            self.reversed = a > b;
        }
    }
    fn replace(&mut self, range: Range<usize>, text: &str) {
        let text = single_line_text(text);
        if self.content[range.clone()] != text {
            if self.marked_range.is_none() {
                // shortcut: retain 100 individual edits; coalesce typing if longer undo history is needed.
                if self.undo.len() == 100 {
                    self.undo.remove(0);
                }
                self.undo.push(self.snapshot());
            }
            self.redo.clear();
            self.content.replace_range(range.clone(), &text);
        }
        let at = range.start + text.len();
        self.selection = at..at;
        self.reversed = false;
    }
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            content: self.content.clone(),
            selection: self.selection.clone(),
            reversed: self.reversed,
        }
    }
    fn restore(&mut self, snapshot: Snapshot) {
        self.content = snapshot.content;
        self.selection = snapshot.selection;
        self.reversed = snapshot.reversed;
        self.marked_range = None;
    }
    pub(crate) fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub(crate) fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
    pub(crate) fn undo(&mut self) {
        if let Some(snapshot) = self.undo.pop() {
            self.redo.push(self.snapshot());
            self.restore(snapshot);
        }
    }
    pub(crate) fn redo(&mut self) {
        if let Some(snapshot) = self.redo.pop() {
            self.undo.push(self.snapshot());
            self.restore(snapshot);
        }
    }
}

fn floor_boundary(text: &str, at: usize) -> usize {
    let mut at = at.min(text.len());
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}

fn utf16_to_utf8(text: &str, at: usize) -> usize {
    let mut utf16 = 0;
    for (index, ch) in text.char_indices() {
        if utf16 >= at {
            return index;
        }
        utf16 += ch.len_utf16();
    }
    text.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_crosses_anchor_and_plain_arrows_collapse_it() {
        let mut edit = TextEdit::new("abcd");
        edit.move_cursor(Movement::Start, false);
        edit.move_cursor(Movement::Right, false);
        edit.move_cursor(Movement::Right, true);
        assert_eq!(edit.selected_text(), "b");
        edit.move_cursor(Movement::Left, true);
        edit.move_cursor(Movement::Left, true);
        assert_eq!(edit.selected_text(), "a");
        assert!(edit.reversed);
        edit.move_cursor(Movement::Right, false);
        assert_eq!(edit.selection, 1..1);
        edit.move_cursor(Movement::End, true);
        assert_eq!(edit.selected_text(), "bcd");
    }

    #[test]
    fn grapheme_deletion_keeps_combining_marks_and_emoji_together() {
        let mut edit = TextEdit::new("写真e\u{301}👩🏽‍💻");
        edit.delete(Movement::Left);
        assert_eq!(edit.content, "写真e\u{301}");
        edit.delete(Movement::Left);
        assert_eq!(edit.content, "写真");
        edit.move_cursor(Movement::Start, false);
        edit.delete(Movement::Right);
        assert_eq!(edit.content, "真");
        edit.undo();
        assert_eq!(edit.content, "写真");
    }

    #[test]
    fn word_navigation_deletion_and_double_click_use_unicode_boundaries() {
        let mut edit = TextEdit::new("/home/写真 album");
        edit.move_cursor(Movement::WordLeft, true);
        assert_eq!(edit.selected_text(), "album");
        edit.replace_utf16(None, "trip");
        edit.delete(Movement::WordLeft);
        assert_eq!(edit.content, "/home/写真 ");
        edit.move_cursor(Movement::Start, false);
        edit.delete(Movement::WordRight);
        assert_eq!(edit.content, "/写真 ");
        edit.select_word(1);
        assert!(!edit.selection.is_empty());
        let mut ascii = TextEdit::new("one two");
        ascii.select_word(5);
        assert_eq!(ascii.selected_text(), "two");
        ascii.select_word(ascii.content.len());
        assert_eq!(ascii.selected_text(), "two");
    }

    #[test]
    fn clipboard_edits_and_undo_redo_restore_text_and_selection() {
        let mut edit = TextEdit::new("before");
        edit.select_all();
        let copied = edit.selected_text().to_owned();
        edit.replace_utf16(None, "");
        assert_eq!(edit.content, "");
        edit.replace_utf16(None, &copied);
        edit.undo();
        assert_eq!(edit.content, "");
        edit.undo();
        assert_eq!(edit.content, "before");
        assert_eq!(edit.selected_text(), "before");
        edit.redo();
        assert_eq!(edit.content, "");
        edit.replace_utf16(None, "new\r\n写真\tname\u{2028}x");
        assert_eq!(edit.content, "new  写真 name x");
        assert!(!edit.can_redo());
        assert!(TextEdit::new("reset").undo.is_empty());
    }

    #[test]
    fn ime_updates_replace_marked_text_and_use_insert_relative_utf16_selection() {
        let mut edit = TextEdit::new("prefix tail");
        edit.move_to(7, false);
        edit.compose_utf16(None, "😀に", Some(2..3));
        assert_eq!(edit.content, "prefix 😀にtail");
        assert_eq!(edit.selected_text(), "に");
        edit.compose_utf16(None, "😀日本", Some(2..4));
        assert_eq!(edit.content, "prefix 😀日本tail");
        assert_eq!(edit.selected_text(), "日本");
        edit.replace_utf16(None, "日本");
        assert_eq!(edit.content, "prefix 日本tail");
        assert!(edit.marked_range.is_none());
        edit.undo();
        assert_eq!(edit.content, "prefix tail");
        edit.redo();
        assert_eq!(edit.content, "prefix 日本tail");
    }

    #[test]
    fn external_utf16_ranges_and_history_are_bounded() {
        let mut edit = TextEdit::new("a😀b");
        assert_eq!(edit.utf8_to_utf16(5), 3);
        edit.replace_utf16(Some(1..3), "写真");
        assert_eq!(edit.content, "a写真b");
        edit.replace_utf16(Some(999..1000), "!");
        assert_eq!(edit.content, "a写真b!");
        for _ in 0..200 {
            edit.replace_utf16(None, "x");
        }
        assert_eq!(edit.undo.len(), 100);
    }
}

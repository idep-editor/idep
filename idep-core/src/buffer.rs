use lsp_types::{CompletionItem, CompletionTextEdit};
use ropey::Rope;
use std::fmt;
use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    /// Zero-based line index
    pub line: usize,
    /// Zero-based column (grapheme index) within the line
    pub column: usize,
}

pub struct Buffer {
    rope: Rope,
    cursor: Cursor,
}

impl Buffer {
    pub fn new() -> Self {
        Self {
            rope: Rope::new(),
            cursor: Cursor { line: 0, column: 0 },
        }
    }

    pub fn with_text(text: &str) -> Self {
        Self {
            rope: Rope::from_str(text),
            cursor: Cursor { line: 0, column: 0 },
        }
    }

    /// Insert text at the given character position (clamped to buffer length)
    pub fn insert(&mut self, pos: usize, text: &str) {
        let insert_pos = pos.min(self.rope.len_chars());
        self.rope.insert(insert_pos, text);
        let new_pos = insert_pos + text.chars().count();
        self.update_cursor(new_pos);
    }

    /// Delete a character range [start, end); no-op if invalid range
    pub fn delete(&mut self, range: Range<usize>) {
        let len = self.rope.len_chars();
        if range.start >= range.end || range.start >= len {
            return;
        }
        let end = range.end.min(len);
        self.rope.remove(range.start..end);
        self.update_cursor(range.start);
    }

    pub fn lines(&self) -> Vec<String> {
        let mut lines: Vec<String> = self
            .rope
            .lines()
            .map(|l| l.to_string().trim_end_matches('\n').to_string())
            .collect();
        if lines.last().map(|l: &String| l.is_empty()).unwrap_or(false) {
            lines.pop();
        }
        lines
    }

    pub fn cursor(&self) -> Cursor {
        self.cursor
    }

    /// Move cursor to end of buffer (last line, after last char).
    pub fn move_cursor_to_end(&mut self) {
        let len = self.rope.len_chars();
        if len == 0 {
            self.cursor = Cursor { line: 0, column: 0 };
            return;
        }
        let line = self.rope.char_to_line(len);
        let line_start = self.rope.line_to_char(line);
        let col = len.saturating_sub(line_start);
        self.cursor = Cursor { line, column: col };
    }

    /// Compute the absolute character index from the current cursor position.
    pub fn cursor_char_index(&self) -> usize {
        let line_start = self.rope.line_to_char(
            self.cursor
                .line
                .min(self.rope.len_lines().saturating_sub(1)),
        );
        line_start
            + self
                .cursor
                .column
                .min(self.rope.line(self.cursor.line).len_chars())
    }

    /// Set cursor to a specific (line, column) position.
    /// Line and column are clamped to valid bounds.
    pub fn set_cursor(&mut self, line: usize, column: usize) {
        let max_line = self.rope.len_lines().saturating_sub(1);
        let line = line.min(max_line);
        let line_len = self.rope.line(line).len_chars();
        // Clamp column to last character (allow cursor past last char for insertion)
        let max_col = if line == max_line && self.rope.len_chars() > 0 {
            line_len
        } else {
            line_len.saturating_sub(1)
        };
        self.cursor = Cursor {
            line,
            column: column.min(max_col),
        };
    }

    /// Apply a text edit: delete the range and insert new text.
    pub fn apply_text_edit(&mut self, range: lsp_types::Range, new_text: &str) {
        let start_char =
            self.rope.line_to_char(range.start.line as usize) + range.start.character as usize;
        let end_char =
            self.rope.line_to_char(range.end.line as usize) + range.end.character as usize;
        self.delete(start_char..end_char);
        self.insert(start_char, new_text);
    }

    /// Apply a completion item at the current cursor position.
    /// Uses textEdit (with range replacement) if present, otherwise insertText or label.
    pub fn apply_completion(&mut self, item: &CompletionItem) {
        if let Some(edit) = &item.text_edit {
            match edit {
                CompletionTextEdit::Edit(e) => {
                    self.apply_text_edit(e.range, &e.new_text);
                    return;
                }
                CompletionTextEdit::InsertAndReplace(e) => {
                    self.apply_text_edit(e.insert, &e.new_text);
                    return;
                }
            }
        }
        let text = item.insert_text.as_deref().unwrap_or(item.label.as_str());
        let pos = self.cursor_char_index();
        self.insert(pos, text);
    }

    fn update_cursor(&mut self, pos: usize) {
        let len = self.rope.len_chars();
        let pos = pos.min(len);
        let line = self.rope.char_to_line(pos);
        // Handle the case where pos == len (one-past-last-char)
        let line = line.min(self.rope.len_lines().saturating_sub(1));
        let line_start = self.rope.line_to_char(line);
        let col = pos.saturating_sub(line_start);
        // Clamp column to valid range (allow cursor at end for insertion)
        let line_len = self.rope.line(line).len_chars();
        let is_last_line = line == self.rope.len_lines().saturating_sub(1);
        let max_col = if is_last_line {
            line_len // Allow cursor past last char on last line
        } else {
            line_len.saturating_sub(1) // Clamp before \n for non-last lines
        };
        self.cursor.line = line;
        self.cursor.column = col.min(max_col);
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Buffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.rope)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_types::TextEdit;

    #[test]
    fn inserts_text_and_updates_cursor() {
        let mut buf = Buffer::with_text("hello");
        buf.insert(5, " world");
        assert_eq!(buf.to_string(), "hello world");
        let cursor = buf.cursor();
        assert_eq!(cursor.line, 0);
        // Cursor positioned after inserted text (one past last char for insertion)
        assert_eq!(cursor.column, 11);
    }

    #[test]
    fn deletes_range_and_clamps_cursor() {
        let mut buf = Buffer::with_text("hello\nworld");
        // remove "hello\n"
        buf.delete(0..6);
        assert_eq!(buf.to_string(), "world");
        let cursor = buf.cursor();
        assert_eq!(cursor.line, 0);
        assert!(cursor.column <= "world".chars().count());
    }

    #[test]
    fn lines_iterates_strings() {
        let buf = Buffer::with_text("a\nb\n");
        let lines = buf.lines();
        assert_eq!(lines, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn delete_on_empty_buffer_is_safe() {
        let mut buf = Buffer::with_text("abc");
        buf.delete(0..3);
        assert_eq!(buf.to_string(), "");
        let cursor = buf.cursor();
        assert_eq!(cursor.line, 0);
        assert_eq!(cursor.column, 0);
    }

    #[test]
    fn apply_completion_prefers_insert_text() {
        let mut buf = Buffer::with_text("foo");
        buf.move_cursor_to_end();

        let item = CompletionItem {
            label: "ignored".into(),
            insert_text: Some("bar".into()),
            ..Default::default()
        };

        buf.apply_completion(&item);
        assert_eq!(buf.to_string(), "foobar");
    }

    #[test]
    fn apply_completion_falls_back_to_label() {
        let mut buf = Buffer::with_text("");
        let item = CompletionItem {
            label: "baz".into(),
            ..Default::default()
        };

        buf.apply_completion(&item);
        assert_eq!(buf.to_string(), "baz");
    }

    #[test]
    fn apply_completion_prefers_text_edit_new_text() {
        let mut buf = Buffer::with_text("hi ");
        buf.move_cursor_to_end();

        // Range covering "hi " (line 0, chars 0-3)
        let edit = CompletionTextEdit::Edit(TextEdit {
            range: lsp_types::Range {
                start: lsp_types::Position {
                    line: 0,
                    character: 0,
                },
                end: lsp_types::Position {
                    line: 0,
                    character: 3,
                },
            },
            new_text: "there".into(),
        });

        let item = CompletionItem {
            label: "ignored".into(),
            text_edit: Some(edit),
            insert_text: Some("fallback".into()),
            ..Default::default()
        };

        buf.apply_completion(&item);
        assert_eq!(buf.to_string(), "there");
    }

    #[test]
    fn apply_completion_at_buffer_start() {
        let mut buf = Buffer::with_text("world");
        // move cursor to start via zero-length insert
        buf.insert(0, "");

        let item = CompletionItem {
            label: "hello ".into(),
            ..Default::default()
        };

        buf.apply_completion(&item);
        assert_eq!(buf.to_string(), "hello world");
    }

    #[test]
    fn apply_completion_at_middle_position() {
        let mut buf = Buffer::with_text("foo bar");
        // place cursor after "foo"
        buf.insert(3, "");

        let item = CompletionItem {
            label: "baz".into(),
            ..Default::default()
        };

        buf.apply_completion(&item);
        assert_eq!(buf.to_string(), "foobaz bar");
    }
}

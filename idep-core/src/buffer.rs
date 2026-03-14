use lsp_types::CompletionItem;
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

    fn cursor_char_index(&self) -> usize {
        let line_start = self.rope.line_to_char(self.cursor.line);
        line_start + self.cursor.column
    }

    /// Apply a completion item at the current cursor position.
    /// Uses `insertText` if present, otherwise falls back to label.
    pub fn apply_completion(&mut self, item: &CompletionItem) {
        let text = item.insert_text.as_deref().unwrap_or(item.label.as_str());
        let pos = self.cursor_char_index();
        self.insert(pos, text);
    }

    fn update_cursor(&mut self, pos: usize) {
        let line = self.rope.char_to_line(pos.min(self.rope.len_chars()));
        let col = pos.saturating_sub(self.rope.line_to_char(line));
        // Clamp column to last character index (trim trailing newline if present)
        let mut line_len = self.rope.line(line).len_chars();
        if self.rope.line(line).chars().last() == Some('\n') {
            line_len = line_len.saturating_sub(1);
        }
        let max_col = line_len.saturating_sub(1);
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

    #[test]
    fn inserts_text_and_updates_cursor() {
        let mut buf = Buffer::with_text("hello");
        buf.insert(5, " world");
        assert_eq!(buf.to_string(), "hello world");
        let cursor = buf.cursor();
        assert_eq!(cursor.line, 0);
        assert_eq!(cursor.column, "hello world".chars().count() - 1);
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
}

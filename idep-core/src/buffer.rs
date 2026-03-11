use ropey::Rope;
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
        let mut lines: Vec<String> = self.rope.lines().map(|l| l.to_string()).collect();
        if lines.last().map(|l| l.is_empty()).unwrap_or(false) {
            lines.pop();
        }
        lines
    }

    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        self.rope.to_string()
    }

    pub fn cursor(&self) -> Cursor {
        self.cursor
    }

    fn update_cursor(&mut self, pos: usize) {
        let line = self.rope.char_to_line(pos.min(self.rope.len_chars()));
        let col = pos.saturating_sub(self.rope.line_to_char(line));
        // Clamp column to line length (without trailing newline)
        let line_len = self.rope.line(line).len_chars().saturating_sub(1);
        self.cursor.line = line;
        self.cursor.column = col.min(line_len);
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Self::new()
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
        assert_eq!(lines, vec!["a\n".to_string(), "b\n".to_string()]);
    }
}

use lsp_types::{CompletionItem, CompletionTextEdit};
use ropey::Rope;
use std::fmt;
use std::ops::Range;

/// Represents a single edit operation for undo/redo.
#[derive(Debug, Clone)]
struct Edit {
    /// The text that was replaced (for undo)
    old_text: String,
    /// The text that was inserted (for redo)
    new_text: String,
    /// The character position where the edit occurred
    start_pos: usize,
    /// The cursor position before the edit
    cursor_before: Cursor,
    /// The cursor position after the edit
    cursor_after: Cursor,
}

impl Edit {
    fn new(
        old_text: String,
        new_text: String,
        start_pos: usize,
        cursor_before: Cursor,
        cursor_after: Cursor,
    ) -> Self {
        Self {
            old_text,
            new_text,
            start_pos,
            cursor_before,
            cursor_after,
        }
    }
}

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
    undo_stack: Vec<Edit>,
    redo_stack: Vec<Edit>,
    max_history: usize,
}

impl Buffer {
    pub fn new() -> Self {
        Self {
            rope: Rope::new(),
            cursor: Cursor { line: 0, column: 0 },
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history: 100,
        }
    }

    pub fn with_text(text: &str) -> Self {
        Self {
            rope: Rope::from_str(text),
            cursor: Cursor { line: 0, column: 0 },
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history: 100,
        }
    }

    /// Insert text at the given character position (clamped to buffer length)
    pub fn insert(&mut self, pos: usize, text: &str) {
        let insert_pos = pos.min(self.rope.len_chars());
        let cursor_before = self.cursor;

        self.rope.insert(insert_pos, text);

        let new_pos = insert_pos + text.chars().count();
        self.update_cursor(new_pos);

        // Record edit for undo: old_text is empty (insertion), new_text is what we inserted
        let edit = Edit::new(
            String::new(),
            text.to_string(),
            insert_pos,
            cursor_before,
            self.cursor,
        );
        self.undo_stack.push(edit);
        self.redo_stack.clear();
        self.maintain_history_limit();
    }

    /// Delete a character range [start, end); no-op if invalid range
    pub fn delete(&mut self, range: Range<usize>) {
        let len = self.rope.len_chars();
        if range.start >= range.end || range.start >= len {
            return;
        }
        let end = range.end.min(len);
        let cursor_before = self.cursor;

        // Capture the text being deleted for undo
        let old_text = self.rope.slice(range.start..end).to_string();

        self.rope.remove(range.start..end);
        self.update_cursor(range.start);

        // Record edit for undo: old_text is what we deleted, new_text is empty
        let edit = Edit::new(
            old_text,
            String::new(),
            range.start,
            cursor_before,
            self.cursor,
        );
        self.undo_stack.push(edit);
        self.redo_stack.clear();
        self.maintain_history_limit();
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

    /// Returns true if there are operations that can be undone.
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Returns true if there are operations that can be redone.
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Returns the maximum number of edits to keep in history.
    pub fn max_history(&self) -> usize {
        self.max_history
    }

    /// Sets the maximum number of edits to keep in history.
    pub fn set_max_history(&mut self, max: usize) {
        self.max_history = max.max(1); // At least 1
        self.maintain_history_limit();
    }

    /// Undo the last edit operation.
    /// Returns true if an operation was undone.
    pub fn undo(&mut self) -> bool {
        if let Some(edit) = self.undo_stack.pop() {
            // Remove the text that was inserted
            let end_pos = edit.start_pos + edit.new_text.len();
            self.rope.remove(edit.start_pos..end_pos);

            // Restore the old text
            if !edit.old_text.is_empty() {
                self.rope.insert(edit.start_pos, &edit.old_text);
            }

            // Restore cursor position
            self.cursor = edit.cursor_before;

            // Push to redo stack
            self.redo_stack.push(edit);
            true
        } else {
            false
        }
    }

    /// Redo the last undone edit operation.
    /// Returns true if an operation was redone.
    pub fn redo(&mut self) -> bool {
        if let Some(edit) = self.redo_stack.pop() {
            // Remove old text if any
            if !edit.old_text.is_empty() {
                let end_pos = edit.start_pos + edit.old_text.len();
                self.rope.remove(edit.start_pos..end_pos);
            }

            // Insert new text
            if !edit.new_text.is_empty() {
                self.rope.insert(edit.start_pos, &edit.new_text);
            }

            // Restore cursor position
            self.cursor = edit.cursor_after;

            // Push back to undo stack
            self.undo_stack.push(edit);
            true
        } else {
            false
        }
    }

    /// Clears all undo and redo history.
    pub fn clear_history(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    /// Remove old entries if we exceed the max history size.
    fn maintain_history_limit(&mut self) {
        while self.undo_stack.len() > self.max_history {
            self.undo_stack.remove(0);
        }
        // Also limit redo stack to prevent unbounded growth
        while self.redo_stack.len() > self.max_history {
            self.redo_stack.remove(0);
        }
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

    // Undo/Redo Tests

    #[test]
    fn undo_insert_restores_previous_state() {
        let mut buf = Buffer::with_text("hello");
        buf.set_cursor(0, 5); // Position cursor at end of "hello"
        buf.insert(5, " world");
        assert_eq!(buf.to_string(), "hello world");

        assert!(buf.undo());
        assert_eq!(buf.to_string(), "hello");
        let cursor = buf.cursor();
        assert_eq!(cursor.column, 5); // Cursor restored to before insert
    }

    #[test]
    fn undo_delete_restores_deleted_text() {
        let mut buf = Buffer::with_text("hello world");
        buf.delete(5..11); // delete " world"
        assert_eq!(buf.to_string(), "hello");

        assert!(buf.undo());
        assert_eq!(buf.to_string(), "hello world");
    }

    #[test]
    fn redo_reapplies_undone_edit() {
        let mut buf = Buffer::with_text("hello");
        buf.insert(5, " world");
        assert!(buf.undo());
        assert_eq!(buf.to_string(), "hello");

        assert!(buf.redo());
        assert_eq!(buf.to_string(), "hello world");
        let cursor = buf.cursor();
        assert_eq!(cursor.column, 11); // Cursor at end after redo
    }

    #[test]
    fn undo_redo_sequence_correct_after_series_of_edits() {
        let mut buf = Buffer::new();

        // Series of edits: "a", "b", "c"
        buf.insert(0, "a");
        buf.insert(1, "b");
        buf.insert(2, "c");
        assert_eq!(buf.to_string(), "abc");

        // Undo all
        assert!(buf.undo());
        assert_eq!(buf.to_string(), "ab");
        assert!(buf.undo());
        assert_eq!(buf.to_string(), "a");
        assert!(buf.undo());
        assert_eq!(buf.to_string(), "");
        assert!(!buf.undo()); // Nothing left to undo

        // Redo all
        assert!(buf.redo());
        assert_eq!(buf.to_string(), "a");
        assert!(buf.redo());
        assert_eq!(buf.to_string(), "ab");
        assert!(buf.redo());
        assert_eq!(buf.to_string(), "abc");
        assert!(!buf.redo()); // Nothing left to redo
    }

    #[test]
    fn new_edit_clears_redo_stack() {
        let mut buf = Buffer::with_text("hello");
        buf.insert(5, " world");
        assert!(buf.undo());
        assert!(buf.can_redo());

        // New edit should clear redo stack
        buf.insert(5, "!");
        assert!(!buf.can_redo());

        // Redo should fail now
        assert!(!buf.redo());
        assert_eq!(buf.to_string(), "hello!");
    }

    #[test]
    fn can_undo_and_can_redo_report_correctly() {
        let mut buf = Buffer::with_text("x");

        assert!(!buf.can_undo());
        assert!(!buf.can_redo());

        buf.insert(1, "y");
        assert!(buf.can_undo());
        assert!(!buf.can_redo());

        buf.undo();
        assert!(!buf.can_undo());
        assert!(buf.can_redo());
    }

    #[test]
    fn max_history_limits_undo_stack() {
        let mut buf = Buffer::new();
        buf.set_max_history(3);

        // Make 5 edits
        for i in 0..5 {
            buf.insert(buf.to_string().len(), &i.to_string());
        }
        assert_eq!(buf.to_string(), "01234");

        // Can only undo 3 (the max_history)
        assert!(buf.undo());
        assert_eq!(buf.to_string(), "0123");
        assert!(buf.undo());
        assert_eq!(buf.to_string(), "012");
        assert!(buf.undo());
        assert_eq!(buf.to_string(), "01");
        assert!(!buf.undo()); // Can't undo further (4 was dropped)
    }

    #[test]
    fn undo_cursor_position_restored_correctly() {
        let mut buf = Buffer::with_text("hello world");
        // Position cursor at start
        buf.set_cursor(0, 0);
        buf.insert(0, "start ");

        let cursor_before = buf.cursor();
        assert_eq!(cursor_before.column, 6);

        buf.undo();
        let cursor_after = buf.cursor();
        assert_eq!(cursor_after.column, 0); // Back to start
    }

    #[test]
    fn clear_history_empties_both_stacks() {
        let mut buf = Buffer::with_text("hello");
        buf.set_cursor(0, 5);
        buf.insert(5, " world");
        assert!(buf.can_undo());
        buf.undo();

        assert!(buf.can_redo()); // Can redo the undone edit
        assert!(!buf.can_undo()); // Nothing left to undo

        buf.clear_history();

        assert!(!buf.can_undo());
        assert!(!buf.can_redo());
    }

    #[test]
    fn undo_multiline_delete() {
        let mut buf = Buffer::with_text("line1\nline2\nline3");
        buf.delete(6..12); // Delete "line2\n"
        assert_eq!(buf.to_string(), "line1\nline3");

        assert!(buf.undo());
        assert_eq!(buf.to_string(), "line1\nline2\nline3");
    }
}

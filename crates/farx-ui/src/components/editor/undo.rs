use super::{EditorState, UndoEntry};

impl EditorState {
    pub(super) fn save_undo(&mut self) {
        self.undo_stack.push(UndoEntry {
            lines: self.lines.clone(),
            cursor_line: self.cursor_line,
            cursor_col: self.cursor_col,
        });
        self.redo_stack.clear();
        // Limit undo history
        if self.undo_stack.len() > 1000 {
            self.undo_stack.remove(0);
        }
    }

    pub(super) fn undo(&mut self) {
        if let Some(entry) = self.undo_stack.pop() {
            self.redo_stack.push(UndoEntry {
                lines: self.lines.clone(),
                cursor_line: self.cursor_line,
                cursor_col: self.cursor_col,
            });
            self.lines = entry.lines;
            self.cursor_line = entry.cursor_line;
            self.cursor_col = entry.cursor_col;
            self.modified = self.undo_stack.len() != self.last_save_undo_len;
        }
    }

    pub(super) fn redo(&mut self) {
        if let Some(entry) = self.redo_stack.pop() {
            self.undo_stack.push(UndoEntry {
                lines: self.lines.clone(),
                cursor_line: self.cursor_line,
                cursor_col: self.cursor_col,
            });
            self.lines = entry.lines;
            self.cursor_line = entry.cursor_line;
            self.cursor_col = entry.cursor_col;
            self.modified = self.undo_stack.len() != self.last_save_undo_len;
        }
    }
}

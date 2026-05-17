use super::EditorState;

impl EditorState {
    pub(super) fn current_line(&self) -> &str {
        self.lines
            .get(self.cursor_line)
            .map(|s| s.as_str())
            .unwrap_or("")
    }

    pub(super) fn current_line_len(&self) -> usize {
        self.current_line().len()
    }

    pub(super) fn clamp_cursor_col(&mut self) {
        let len = self.current_line_len();
        if self.cursor_col > len {
            self.cursor_col = len;
        }
        // Ensure we're at a char boundary
        if self.cursor_line < self.lines.len() {
            while self.cursor_col > 0
                && !self.lines[self.cursor_line].is_char_boundary(self.cursor_col)
            {
                self.cursor_col -= 1;
            }
        }
    }

    pub(super) fn insert_char(&mut self, ch: char) {
        self.save_undo();
        if self.cursor_line < self.lines.len() {
            self.lines[self.cursor_line].insert(self.cursor_col, ch);
            self.cursor_col += ch.len_utf8();
        }
        self.modified = true;
    }

    pub(super) fn insert_newline(&mut self) {
        self.save_undo();
        if self.cursor_line < self.lines.len() {
            // Ensure cursor_col is at a char boundary
            let line = &self.lines[self.cursor_line];
            let col = self.cursor_col.min(line.len());
            let col = if col > 0 && !line.is_char_boundary(col) {
                line.char_indices()
                    .rev()
                    .find(|&(i, _)| i <= col)
                    .map(|(i, _)| i)
                    .unwrap_or(0)
            } else {
                col
            };
            let rest = self.lines[self.cursor_line][col..].to_string();
            self.lines[self.cursor_line].truncate(col);
            self.cursor_line += 1;
            self.lines.insert(self.cursor_line, rest);
            self.cursor_col = 0;
        }
        self.modified = true;
    }

    pub(super) fn backspace(&mut self) {
        if self.cursor_col > 0 {
            self.save_undo();
            self.cursor_col -= 1;
            // Handle multi-byte chars
            while self.cursor_col > 0
                && !self.lines[self.cursor_line].is_char_boundary(self.cursor_col)
            {
                self.cursor_col -= 1;
            }
            self.lines[self.cursor_line].remove(self.cursor_col);
            self.modified = true;
        } else if self.cursor_line > 0 {
            self.save_undo();
            let current = self.lines.remove(self.cursor_line);
            self.cursor_line -= 1;
            self.cursor_col = self.lines[self.cursor_line].len();
            self.lines[self.cursor_line].push_str(&current);
            self.modified = true;
        }
    }

    pub(super) fn delete_char(&mut self) {
        let line_len = self.current_line_len();
        if self.cursor_col < line_len {
            self.save_undo();
            self.lines[self.cursor_line].remove(self.cursor_col);
            self.modified = true;
        } else if self.cursor_line + 1 < self.lines.len() {
            self.save_undo();
            let next = self.lines.remove(self.cursor_line + 1);
            self.lines[self.cursor_line].push_str(&next);
            self.modified = true;
        }
    }
}

use super::EditorState;

impl EditorState {
    pub(super) fn find_next(&mut self) {
        if self.search_query.is_empty() || self.lines.is_empty() {
            return;
        }
        let query = self.search_query.clone();
        // Search from current position forward
        let start_line = self.cursor_line.min(self.lines.len() - 1);
        let start_col = self.cursor_col + 1;

        for i in 0..self.lines.len() {
            let line_idx = (start_line + i) % self.lines.len();
            let search_from = if i == 0 {
                // Ensure search_from is at a char boundary
                let line = &self.lines[line_idx];
                let mut sf = start_col.min(line.len());
                while sf > 0 && sf < line.len() && !line.is_char_boundary(sf) {
                    sf += 1;
                }
                sf
            } else {
                0
            };
            if search_from <= self.lines[line_idx].len() {
                if let Some(pos) = self.lines[line_idx][search_from..].find(&query) {
                    self.cursor_line = line_idx;
                    self.cursor_col = search_from + pos;
                    return;
                }
            }
        }
    }
}

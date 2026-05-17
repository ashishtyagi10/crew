use super::state::ViewerState;

impl ViewerState {
    pub(super) fn find_in_viewer(&mut self, query: &str) {
        if query.is_empty() {
            return;
        }
        let query_lower = query.to_lowercase();
        // Search from current scroll position forward
        for i in self.scroll_offset..self.total_lines {
            if i < self.lines.len() && self.lines[i].to_lowercase().contains(&query_lower) {
                self.scroll_offset = i;
                self.search = Some(query.to_string());
                return;
            }
        }
        // Wrap around from beginning
        for i in 0..self.scroll_offset {
            if i < self.lines.len() && self.lines[i].to_lowercase().contains(&query_lower) {
                self.scroll_offset = i;
                self.search = Some(query.to_string());
                return;
            }
        }
    }

    pub(super) fn find_next_in_viewer(&mut self) {
        if let Some(query) = self.search.clone() {
            let query_lower = query.to_lowercase();
            for i in (self.scroll_offset + 1)..self.total_lines {
                if i < self.lines.len() && self.lines[i].to_lowercase().contains(&query_lower) {
                    self.scroll_offset = i;
                    return;
                }
            }
            // Wrap
            for i in 0..=self.scroll_offset {
                if i < self.lines.len() && self.lines[i].to_lowercase().contains(&query_lower) {
                    self.scroll_offset = i;
                    return;
                }
            }
        }
    }
}

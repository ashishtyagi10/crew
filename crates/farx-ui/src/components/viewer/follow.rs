use super::state::ViewerState;

impl ViewerState {
    /// Reload file contents (for follow/tail mode). Returns true if content changed.
    pub fn reload_if_follow(&mut self) -> bool {
        if !self.follow || self.hex_mode {
            return false;
        }
        let Ok(contents) = std::fs::read_to_string(&self.file_path) else {
            return false;
        };
        let new_lines: Vec<String> = contents.lines().map(String::from).collect();
        if new_lines.len() == self.lines.len() {
            return false;
        }
        self.lines = new_lines;
        self.total_lines = self.lines.len();
        self.scroll_offset = self.total_lines.saturating_sub(self.visible_height.max(1));
        true
    }
}

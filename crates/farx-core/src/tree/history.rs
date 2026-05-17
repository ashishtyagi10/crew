use super::types::TreeState;
use std::path::PathBuf;

impl TreeState {
    /// Change root directory (no history tracking — used for init/reset).
    pub fn set_root(&mut self, root: PathBuf) {
        self.root = root.clone();
        self.expanded.clear();
        self.expanded.insert(root);
        self.cursor = 0;
        self.scroll_offset = 0;
        self.selected.clear();
        self.filter.clear();
        self.rebuild();
        self.refresh_git_status();
    }

    /// Navigate to a new directory, pushing current location to history.
    pub fn navigate_to(&mut self, target: PathBuf) {
        if target == self.root {
            return;
        }
        // Push current to back-stack
        self.history_back.push(self.root.clone());
        // Clear forward stack on new navigation
        self.history_forward.clear();
        self.set_root(target);
    }

    /// Go back to the previous directory in history.
    /// Returns true if navigation occurred.
    pub fn go_back(&mut self) -> bool {
        if let Some(prev) = self.history_back.pop() {
            self.history_forward.push(self.root.clone());
            self.set_root(prev);
            true
        } else {
            false
        }
    }

    /// Go forward in history.
    /// Returns true if navigation occurred.
    pub fn go_forward(&mut self) -> bool {
        if let Some(next) = self.history_forward.pop() {
            self.history_back.push(self.root.clone());
            self.set_root(next);
            true
        } else {
            false
        }
    }
}

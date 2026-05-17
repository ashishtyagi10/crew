use super::types::{TreeNode, TreeState};

impl TreeState {
    /// Get current node
    pub fn current_node(&self) -> Option<&TreeNode> {
        self.visible_nodes.get(self.cursor)
    }

    /// Toggle expand/collapse of the directory at cursor
    pub fn toggle_expand(&mut self) {
        if let Some(node) = self.visible_nodes.get(self.cursor) {
            if node.entry.is_dir {
                let path = node.entry.path.clone();
                if self.expanded.contains(&path) {
                    self.expanded.remove(&path);
                } else {
                    self.expanded.insert(path);
                }
                self.rebuild();
            }
        }
    }

    /// Right arrow: expand collapsed dir, or move into first child of expanded dir.
    pub fn expand(&mut self) {
        let Some(node) = self.visible_nodes.get(self.cursor) else {
            return;
        };
        if !node.entry.is_dir {
            return;
        }

        let was_expanded = node.expanded;
        let depth = node.depth;

        if !was_expanded {
            let path = node.entry.path.clone();
            self.expanded.insert(path);
            self.rebuild();
        }

        // Move cursor to first child
        if self.cursor + 1 < self.visible_nodes.len()
            && self.visible_nodes[self.cursor + 1].depth > depth
        {
            self.cursor += 1;
        }
    }

    /// Left arrow: collapse expanded dir, or jump to parent node.
    pub fn collapse(&mut self) {
        if let Some(node) = self.visible_nodes.get(self.cursor) {
            if node.entry.is_dir && node.expanded {
                // Expanded → collapse
                let path = node.entry.path.clone();
                self.expanded.remove(&path);
                self.rebuild();
            } else {
                // Collapsed dir or file → jump to parent directory node
                let current_depth = node.depth;
                if current_depth > 0 {
                    for i in (0..self.cursor).rev() {
                        if self.visible_nodes[i].depth < current_depth
                            && self.visible_nodes[i].entry.is_dir
                        {
                            self.cursor = i;
                            break;
                        }
                    }
                }
            }
        }
    }

    pub fn move_cursor(&mut self, delta: i32) {
        let new_pos = (self.cursor as i32 + delta)
            .max(0)
            .min(self.visible_nodes.len() as i32 - 1) as usize;
        self.cursor = new_pos;
    }

    pub fn move_cursor_to(&mut self, pos: usize) {
        self.cursor = pos.min(self.visible_nodes.len().saturating_sub(1));
    }

    pub fn scroll_to_cursor(&mut self, visible_height: usize) {
        if self.cursor < self.scroll_offset {
            self.scroll_offset = self.cursor;
        }
        if self.cursor >= self.scroll_offset + visible_height {
            self.scroll_offset = self.cursor - visible_height + 1;
        }
    }

    pub fn toggle_select(&mut self) {
        if self.cursor < self.visible_nodes.len() {
            // Skip ".." from selection
            if self.visible_nodes[self.cursor].entry.name == ".." {
                if self.cursor + 1 < self.visible_nodes.len() {
                    self.cursor += 1;
                }
                return;
            }
            if self.selected.contains(&self.cursor) {
                self.selected.remove(&self.cursor);
            } else {
                self.selected.insert(self.cursor);
            }
            if self.cursor + 1 < self.visible_nodes.len() {
                self.cursor += 1;
            }
        }
    }
}

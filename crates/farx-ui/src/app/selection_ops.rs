//! Selection-related operations: mask-based select/deselect, sort field
//! toggling, and the cross-panel directory compare.

use std::collections::HashMap;

use farx_core::{PanelSide, SortField, SortOrder};

use super::globs::glob_match;
use super::App;

impl App {
    /// Select or deselect files matching a glob pattern in the active tree.
    pub(super) fn apply_mask_selection(&mut self, pattern: &str, select: bool) {
        let pat = pattern.to_lowercase();
        let tree = self.active_tree();
        let mut count = 0usize;
        for i in 0..tree.visible_nodes.len() {
            let name = &tree.visible_nodes[i].entry.name;
            if name == ".." {
                continue;
            }
            if glob_match(&pat, &name.to_lowercase()) {
                if select {
                    if tree.selected.insert(i) {
                        count += 1;
                    }
                } else if tree.selected.remove(&i) {
                    count += 1;
                }
            }
        }
        let verb = if select { "Selected" } else { "Deselected" };
        self.feedback
            .info(format!("{} {} file(s) matching '{}'", verb, count, pattern));
    }

    /// Toggle sort: if already sorted by this field, flip asc/desc; otherwise
    /// set the field and reset to ascending.
    pub(super) fn toggle_sort(&mut self, field: SortField) {
        let panel = self.active_panel_mut();
        if panel.sort_field == field {
            panel.sort_order = match panel.sort_order {
                SortOrder::Ascending => SortOrder::Descending,
                SortOrder::Descending => SortOrder::Ascending,
            };
        } else {
            panel.sort_field = field;
            panel.sort_order = SortOrder::Ascending;
        }
        let new_order = panel.sort_order;
        panel.sort_entries();

        let tree = self.active_tree();
        tree.sort_field = field;
        tree.sort_order = new_order;
        tree.rebuild();

        let field_name = match field {
            SortField::Name => "Name",
            SortField::Extension => "Extension",
            SortField::Size => "Size",
            SortField::Modified => "Date",
        };
        let order = match new_order {
            SortOrder::Ascending => "↑",
            SortOrder::Descending => "↓",
        };
        self.feedback
            .info(format!("Sort: {} {}", field_name, order));
    }

    /// Compare directories: select files in the active panel that are unique
    /// to it or differ from the corresponding entry in the other panel.
    pub(super) fn compare_directories(&mut self) {
        let other_tree = match self.active_panel {
            PanelSide::Left => &self.right_tree,
            PanelSide::Right => &self.left_tree,
        };
        let other_files: HashMap<String, (u64, Option<chrono::DateTime<chrono::Local>>)> =
            other_tree
                .visible_nodes
                .iter()
                .filter(|n| n.depth == 0 && n.entry.name != "..")
                .map(|n| (n.entry.name.clone(), (n.entry.size, n.entry.modified)))
                .collect();

        let tree = self.active_tree();
        tree.selected.clear();
        let mut selected_count = 0usize;
        for i in 0..tree.visible_nodes.len() {
            let node = &tree.visible_nodes[i];
            if node.depth != 0 || node.entry.name == ".." {
                continue;
            }
            match other_files.get(&node.entry.name) {
                None => {
                    tree.selected.insert(i);
                    selected_count += 1;
                }
                Some(&(other_size, other_modified)) => {
                    if node.entry.size != other_size || node.entry.modified != other_modified {
                        tree.selected.insert(i);
                        selected_count += 1;
                    }
                }
            }
        }
        self.feedback.info(format!(
            "Compare: {} file(s) differ or are unique",
            selected_count,
        ));
    }
}

//! Panel/tree side dispatch — get the active/inactive tree or panel, the
//! current selection paths/names, and the navigate-to entry point.

use std::path::PathBuf;

use farx_core::{PanelSide, PanelState, TreeState};

use super::App;

impl App {
    /// Get a mutable reference to the currently active panel.
    pub fn active_panel_mut(&mut self) -> &mut PanelState {
        match self.active_panel {
            PanelSide::Left => &mut self.left_panel,
            PanelSide::Right => &mut self.right_panel,
        }
    }

    /// Get the active tree (mutable).
    pub(super) fn active_tree(&mut self) -> &mut TreeState {
        match self.active_panel {
            PanelSide::Left => &mut self.left_tree,
            PanelSide::Right => &mut self.right_tree,
        }
    }

    /// Get the active tree (immutable).
    pub fn active_tree_ref(&self) -> &TreeState {
        match self.active_panel {
            PanelSide::Left => &self.left_tree,
            PanelSide::Right => &self.right_tree,
        }
    }

    /// Get the inactive tree's root directory.
    pub(super) fn inactive_tree_root(&self) -> PathBuf {
        match self.active_panel {
            PanelSide::Left => self.right_tree.root.clone(),
            PanelSide::Right => self.left_tree.root.clone(),
        }
    }

    /// Get a reference to the currently active panel.
    pub fn active_panel_ref(&self) -> &PanelState {
        match self.active_panel {
            PanelSide::Left => &self.left_panel,
            PanelSide::Right => &self.right_panel,
        }
    }

    /// Get a reference to the currently inactive panel.
    pub fn inactive_panel(&self) -> &PanelState {
        match self.active_panel {
            PanelSide::Left => &self.right_panel,
            PanelSide::Right => &self.left_panel,
        }
    }

    /// Collect paths from tree selection (or current node).
    pub(super) fn collect_selected_paths(&self) -> Vec<PathBuf> {
        let tree = self.active_tree_ref();
        if tree.selected.is_empty() {
            if let Some(node) = tree.current_node() {
                return vec![node.entry.path.clone()];
            }
            Vec::new()
        } else {
            tree.selected
                .iter()
                .filter_map(|&i| tree.visible_nodes.get(i))
                .map(|n| n.entry.path.clone())
                .collect()
        }
    }

    /// Collect display names from tree selection.
    pub(super) fn collect_selected_names(&self) -> Vec<String> {
        let tree = self.active_tree_ref();
        if tree.selected.is_empty() {
            if let Some(node) = tree.current_node() {
                return vec![node.entry.name.clone()];
            }
            Vec::new()
        } else {
            tree.selected
                .iter()
                .filter_map(|&i| tree.visible_nodes.get(i))
                .map(|n| n.entry.name.clone())
                .collect()
        }
    }

    /// Navigate the active panel to a new directory.
    pub(super) fn navigate_to(&mut self, path: PathBuf) {
        match self.active_panel {
            PanelSide::Left => {
                self.left_tree.navigate_to(path.clone());
                self.left_panel.current_dir = path;
            }
            PanelSide::Right => {
                self.right_tree.navigate_to(path.clone());
                self.right_panel.current_dir = path;
            }
        }
        self.update_fs_watcher();
    }
}

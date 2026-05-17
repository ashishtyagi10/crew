//! Selection-action routing: per-row toggle, range select up/down/page/home/
//! end, select-all, deselect-all, invert-selection.

use farx_core::Action;

use super::super::App;

impl App {
    /// Handle selection-related actions. Returns `true` if the action was
    /// consumed; the caller should stop further dispatch.
    pub(in crate::app) fn dispatch_selection(&mut self, action: &Action) -> bool {
        match action {
            Action::ToggleSelect => self.active_tree().toggle_select(),
            Action::SelectUp => {
                self.active_tree().toggle_select();
                self.active_tree().move_cursor(-1);
            }
            Action::SelectDown => {
                self.active_tree().toggle_select();
                self.active_tree().move_cursor(1);
            }
            Action::SelectPageUp => self.select_range_up(20),
            Action::SelectPageDown => self.select_range_down(20),
            Action::SelectHome => {
                let cursor = self.active_tree().cursor;
                self.select_range_up(cursor);
            }
            Action::SelectEnd => {
                let tree = self.active_tree();
                let max = tree.visible_nodes.len().saturating_sub(1);
                let cursor = tree.cursor;
                self.select_range_down(max.saturating_sub(cursor));
            }
            Action::SelectAll => {
                let tree = self.active_tree();
                for i in 0..tree.visible_nodes.len() {
                    if tree.visible_nodes[i].entry.name != ".." {
                        tree.selected.insert(i);
                    }
                }
            }
            Action::DeselectAll => self.active_tree().selected.clear(),
            Action::InvertSelection => {
                let tree = self.active_tree();
                for i in 0..tree.visible_nodes.len() {
                    if tree.visible_nodes[i].entry.name != ".." {
                        if tree.selected.contains(&i) {
                            tree.selected.remove(&i);
                        } else {
                            tree.selected.insert(i);
                        }
                    }
                }
            }
            _ => return false,
        }
        true
    }

    fn select_range_up(&mut self, span: usize) {
        let tree = self.active_tree();
        let cursor = tree.cursor;
        let target = cursor.saturating_sub(span);
        for i in (target..cursor).rev() {
            if i < tree.visible_nodes.len() && tree.visible_nodes[i].entry.name != ".." {
                tree.selected.insert(i);
            }
        }
        tree.move_cursor_to(target);
    }

    fn select_range_down(&mut self, span: usize) {
        let tree = self.active_tree();
        let cursor = tree.cursor;
        let max = tree.visible_nodes.len().saturating_sub(1);
        let target = (cursor + span).min(max);
        for i in cursor..=target {
            if i < tree.visible_nodes.len() && tree.visible_nodes[i].entry.name != ".." {
                tree.selected.insert(i);
            }
        }
        tree.move_cursor_to(target);
    }
}

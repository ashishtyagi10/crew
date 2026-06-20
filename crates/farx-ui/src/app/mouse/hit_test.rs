//! Mouse hit-testing against cached panel rects: which side was clicked,
//! which row within the list, and whether the click landed on a breadcrumb.

use std::path::PathBuf;

use farx_core::PanelSide;

use super::super::App;

impl App {
    /// If a click lands on a terminal tile, focus it and promote it in the
    /// grid order. Returns `true` when the click was consumed by a terminal.
    pub(super) fn try_focus_terminal_at(&mut self, x: u16, y: u16) -> bool {
        // Collect matching terminal id without holding an immutable borrow.
        let hit = self.cached_panel_rects.iter().find_map(|(leaf, rect)| {
            if x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height {
                if let farx_core::PanelLeaf::Terminal(id) = leaf {
                    return Some(*id);
                }
            }
            None
        });
        if let Some(id) = hit {
            self.focused_terminal = Some(id);
            self.grid.touch(id);
            return true;
        }
        false
    }

    /// Determine which panel side a screen coordinate falls in.
    pub(super) fn panel_side_at(&self, x: u16, y: u16) -> Option<PanelSide> {
        for (leaf, rect) in &self.cached_panel_rects {
            if x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height {
                if let farx_core::PanelLeaf::FilePanel(side) = leaf {
                    return Some(*side);
                }
            }
        }
        None
    }

    /// Determine panel side and the row index within the file list area.
    /// Returns `(side, Some(row_in_visible_list))` or `(side, None)` if the
    /// click is on the header or footer.
    pub(super) fn panel_row_at(&self, x: u16, y: u16) -> Option<(PanelSide, Option<usize>)> {
        for (leaf, rect) in &self.cached_panel_rects {
            if x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height {
                if let farx_core::PanelLeaf::FilePanel(side) = leaf {
                    let tree = match side {
                        PanelSide::Left => &self.left_tree,
                        PanelSide::Right => &self.right_tree,
                    };
                    let tab_height: u16 = if tree.tab_count() > 1 { 1 } else { 0 };
                    let inner_y = rect.y + tab_height + 1;
                    let is_active_panel = self.active_panel == *side;
                    let filter_height: u16 =
                        if !tree.filter.is_empty() || (self.filter_active && is_active_panel) {
                            1
                        } else {
                            0
                        };
                    let list_start_y = inner_y + filter_height;
                    let list_end_y = rect.y + rect.height - 2;
                    if y >= list_start_y && y < list_end_y {
                        let row = (y - list_start_y) as usize;
                        return Some((*side, Some(row)));
                    }
                    return Some((*side, None));
                }
            }
        }
        None
    }

    /// Check if a click hit a breadcrumb segment in the panel title bar.
    /// Returns `(panel_side, path)` so the caller can switch panels correctly.
    pub(super) fn breadcrumb_hit(&self, x: u16, y: u16) -> Option<(PanelSide, PathBuf)> {
        use crate::components::tree_panel::breadcrumb_path_at_click;
        for (leaf, rect) in &self.cached_panel_rects {
            if y == rect.y && x >= rect.x && x < rect.x + rect.width {
                if let farx_core::PanelLeaf::FilePanel(side) = leaf {
                    let tree = match side {
                        PanelSide::Left => &self.left_tree,
                        PanelSide::Right => &self.right_tree,
                    };
                    if let Some(path) = breadcrumb_path_at_click(&tree.root, *rect, x) {
                        return Some((*side, path));
                    }
                }
            }
        }
        None
    }
}

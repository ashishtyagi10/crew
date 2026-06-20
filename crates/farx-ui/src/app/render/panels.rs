//! Render the active layout's leaf panels (file panels and embedded
//! terminals). File panels also paint a tab bar above the tree view.
//! `render_agent_grid` is the new main-surface renderer (Plan 2 Task 3).

use ratatui::layout::Rect;
use ratatui::Frame;

use farx_core::{PanelLeaf, PanelSide, TabGroup};

use crate::components::embedded_terminal::{render_terminal, render_thumbnail};
use crate::components::info_panel::{render_info_panel, InfoPanelData};
use crate::components::tree_panel::{render_tab_bar, render_tree_panel_with_filter};

use super::super::App;

impl App {
    pub(super) fn render_agent_grid(&mut self, frame: &mut Frame, area: Rect) {
        use farx_core::compute_grid_layout;
        let layout = compute_grid_layout(area, &self.grid);
        self.cached_panel_rects.clear();
        for (id, rect) in &layout.full {
            let inner_h = rect.height.saturating_sub(2);
            let inner_w = rect.width.saturating_sub(2);
            if inner_h > 0 && inner_w > 0 {
                if let Some(term) = self.terminal_by_id_mut(*id) {
                    term.resize(inner_h, inner_w);
                }
            }
            let is_focused = self.focused_terminal == Some(*id);
            if let Some(term) = self.terminal_by_id(*id) {
                render_terminal(frame, *rect, term, is_focused);
                self.cached_panel_rects
                    .push((farx_core::PanelLeaf::Terminal(*id), *rect));
            }
        }
        for (id, rect) in &layout.minimized {
            if let Some(term) = self.terminal_by_id(*id) {
                render_thumbnail(frame, *rect, term);
                self.cached_panel_rects
                    .push((farx_core::PanelLeaf::Terminal(*id), *rect));
            }
        }
    }

    #[allow(dead_code)] // Plan 4: remove
    pub(super) fn render_panel_leaves(
        &mut self,
        frame: &mut Frame,
        panel_rects: &[(PanelLeaf, Rect)],
    ) {
        for (leaf, rect) in panel_rects {
            match leaf {
                PanelLeaf::FilePanel(side) => {
                    let (tabs, tree, panel_state) = match side {
                        PanelSide::Left => (
                            self.left_tree.tab_labels(),
                            &mut self.left_tree as &mut TabGroup,
                            &self.left_panel,
                        ),
                        PanelSide::Right => (
                            self.right_tree.tab_labels(),
                            &mut self.right_tree as &mut TabGroup,
                            &self.right_panel,
                        ),
                    };
                    let is_active = self.focused_terminal.is_none() && self.active_panel == *side;
                    let filter_editing = is_active && self.filter_active;

                    let tab_height = render_tab_bar(frame, *rect, &tabs, is_active, &self.theme);
                    let panel_rect = Rect {
                        y: rect.y + tab_height,
                        height: rect.height.saturating_sub(tab_height),
                        ..*rect
                    };

                    let panel_height = panel_rect.height.saturating_sub(3) as usize;
                    tree.scroll_to_cursor(panel_height);

                    if self.show_info_panel && *side != self.active_panel {
                        let current_file = self.active_tree_ref().current_node().map(|n| &n.entry);
                        let data = InfoPanelData::from_panel(panel_state, current_file);
                        render_info_panel(frame, panel_rect, &data, &self.theme);
                    } else {
                        render_tree_panel_with_filter(
                            frame,
                            panel_rect,
                            tree,
                            is_active,
                            &self.theme,
                            filter_editing,
                        );
                    }
                }
                PanelLeaf::Terminal(tid) => {
                    let inner_h = rect.height.saturating_sub(2);
                    let inner_w = rect.width.saturating_sub(2);
                    if inner_h > 0 && inner_w > 0 {
                        if let Some(term) = self.terminal_by_id_mut(*tid) {
                            term.resize(inner_h, inner_w);
                        }
                    }
                    let is_focused = self.focused_terminal == Some(*tid);
                    if let Some(term) = self.terminal_by_id(*tid) {
                        render_terminal(frame, *rect, term, is_focused);
                    }
                }
            }
        }
    }
}

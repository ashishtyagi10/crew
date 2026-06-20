//! Render the agent grid (file panels and embedded terminals).
//! `render_agent_grid` is the main surface renderer.

use ratatui::layout::Rect;
use ratatui::Frame;

use crate::components::embedded_terminal::{render_terminal, render_thumbnail};

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
}

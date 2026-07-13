//! Pane geometry application: assign pixel rects and resize PTYs when the
//! derived cell grid changes (split from `pane.rs` for the 200-line cap).
use crate::layout::Rect;
use crate::pane::{Pane, PaneContent};
use crew_term::{GridSize, TermModel};

/// Assign one pane's pixel rect and resize its PTY (Terminal only) when the
/// derived grid changes. Reserves a one-cell border ring (fieldset card).
pub fn relayout_one(pane: &mut Pane, rect: Rect, cell_w: f32, cell_h: f32) {
    pane.rect = rect;
    let (cols, rows) = crate::layout::card_inner_cells(rect.w, rect.h, cell_w, cell_h);
    if cols != pane.grid.cols || rows != pane.grid.rows {
        let new_grid = GridSize { cols, rows };
        if let PaneContent::Terminal(t) = &mut pane.content {
            t.pty.resize(new_grid);
        }
        pane.grid = new_grid;
    }
}

/// Assign pixel rects to panes (zipped in order). Thin wrapper over `relayout_one`.
pub fn relayout(panes: &mut [Pane], rects: &[Rect], cell_w: f32, cell_h: f32) {
    for (pane, &rect) in panes.iter_mut().zip(rects.iter()) {
        relayout_one(pane, rect, cell_w, cell_h);
    }
}

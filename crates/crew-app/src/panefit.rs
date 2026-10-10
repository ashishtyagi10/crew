//! Pane geometry application: assign pixel rects and resize PTYs when the
//! derived cell grid changes (split from `pane.rs` for the 200-line cap).
use crate::layout::Rect;
use crate::pane::{Pane, PaneContent};
use crew_term::{GridSize, TermModel};

/// Assign one pane's pixel rect and resize its PTY (Terminal only) when the
/// derived grid changes. Reserves a one-cell border ring (fieldset card).
pub fn relayout_one(pane: &mut Pane, rect: Rect, cell_w: f32, cell_h: f32) {
    pane.rect = rect;
    if let PaneContent::Terminal(t) = &mut pane.content {
        // How big a cell is in pixels decides how many rows a picture the
        // program sends has to reserve (see `crew_term::PlacedImage`).
        t.pty
            .set_cell_px(cell_w.round() as u32, cell_h.round() as u32);
    }
    let (cols, rows) = crate::layout::card_inner_cells(rect.w, rect.h, cell_w, cell_h);
    let cols = cols.saturating_sub(content_inset(&pane.content)).max(1);
    if cols != pane.grid.cols || rows != pane.grid.rows {
        let new_grid = GridSize { cols, rows };
        if let PaneContent::Terminal(t) = &mut pane.content {
            t.pty.resize(new_grid);
        }
        pane.grid = new_grid;
    }
}

/// Columns a pane's content stands in from the first column inside its
/// frame. That column starts half a cell past the frame's stroke: a chat
/// indents its own text one more and reads a cell and a half off the rule,
/// while a shell beside it started on the stroke's heels (glass survey C#6).
/// A terminal takes the column here, so its pty is a column narrower and
/// its cells, clicks and selection move together (`paneview`,
/// `CrewApp::cursor_rowcol`).
pub const TERM_INSET: u16 = 1;

/// [`TERM_INSET`] for a terminal; nothing for a pane that sets its own.
pub fn content_inset(content: &PaneContent) -> u16 {
    match content {
        PaneContent::Terminal(_) => TERM_INSET,
        _ => 0,
    }
}

/// [`content_inset`] in px for cells `cw` wide.
pub fn inset_px(content: &PaneContent, cw: f32) -> f32 {
    f32::from(content_inset(content)) * cw
}

/// Assign pixel rects to panes (zipped in order). Thin wrapper over `relayout_one`.
pub fn relayout(panes: &mut [Pane], rects: &[Rect], cell_w: f32, cell_h: f32) {
    for (pane, &rect) in panes.iter_mut().zip(rects.iter()) {
        relayout_one(pane, rect, cell_w, cell_h);
    }
}

#[cfg(all(test, unix))]
#[path = "terminset_tests.rs"]
mod tests;

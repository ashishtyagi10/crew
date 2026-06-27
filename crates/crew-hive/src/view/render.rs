//! Headless glyph-grid renderer: maps a FleetView into placed CellGlyphs.
//!
//! Edges between constellation nodes are intentionally omitted here; the GPU
//! layer draws lines between node rects using the Constellation edge list.
use crate::view::{FleetView, Rgb};
use serde::{Deserialize, Serialize};

/// A single rendered character at a grid position.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CellGlyph {
    pub col: u16,
    pub row: u16,
    pub ch: char,
    pub color: Rgb,
}

/// Render a `FleetView` into a flat list of placed glyphs.
///
/// Returns an empty vec when `cols == 0 || rows == 0`.
/// All output positions are clamped to `[0, cols-1]` × `[0, rows-1]`.
pub fn render_cells(view: &FleetView, cols: u16, rows: u16) -> Vec<CellGlyph> {
    if cols == 0 || rows == 0 {
        return vec![];
    }
    match view {
        FleetView::Constellation(c) => render_constellation(c, cols, rows),
        FleetView::Heatmap(h) => render_heatmap(h, cols, rows),
    }
}

fn render_constellation(c: &crate::view::Constellation, cols: u16, rows: u16) -> Vec<CellGlyph> {
    c.nodes
        .iter()
        .map(|n| {
            let col = scale_f32(n.x, cols);
            let row = scale_f32(n.y, rows);
            CellGlyph {
                col,
                row,
                ch: '●',
                color: n.color,
            }
        })
        .collect()
}

/// Scale a normalised float `v ∈ [0,1]` into `[0, extent-1]`.
#[inline]
fn scale_f32(v: f32, extent: u16) -> u16 {
    ((v * (extent - 1) as f32).round() as u16).min(extent - 1)
}

fn render_heatmap(h: &crate::view::Heatmap, cols: u16, rows: u16) -> Vec<CellGlyph> {
    let one_to_one = h.cols <= cols as usize && h.rows <= rows as usize;
    let max_vc = h.cols.saturating_sub(1).max(1);
    let max_vr = h.rows.saturating_sub(1).max(1);

    h.cells
        .iter()
        .map(|cell| {
            let (out_col, out_row) = if one_to_one {
                (cell.col as u16, cell.row as u16)
            } else {
                let c = ((cell.col * (cols - 1) as usize) / max_vc) as u16;
                let r = ((cell.row * (rows - 1) as usize) / max_vr) as u16;
                (c.min(cols - 1), r.min(rows - 1))
            };
            CellGlyph {
                col: out_col,
                row: out_row,
                ch: '■',
                color: cell.color,
            }
        })
        .collect()
}

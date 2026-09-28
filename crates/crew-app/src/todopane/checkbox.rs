//! The done control, drawn as a checkbox you can find.
//!
//! The row's mark was `○`/`●` — a font glyph, and in most faces a few pixels
//! across; the user (2026-09-27): "not sure where to click to mark it done…
//! either increase that size or make it checkbox". The cells now carry `☐` /
//! `☑` (crew draws both itself, so every face shows the same box), and where
//! the pane is drawn with its vector layer the box is painted larger than a
//! cell allows: a square [`SIDE`] of a row tall, spanning the columns either
//! side of the mark — all inside the three-cell click target `[ ]` held.
use crew_render::{CellView, Paint};

use super::render::BOX_COL;
use super::TodoPane;

/// The to-do mark in the cells.
pub(crate) const OPEN: char = '\u{2610}';
/// The done mark in the cells.
pub(crate) const DONE: char = '\u{2611}';
/// The painted box's side, as a fraction of the row height — 13 px on a
/// 16 px row, the size a checkbox is on every settings page, with a sliver
/// of air to the box on the next row.
const SIDE: f32 = 0.8;
/// Its outline, as a fraction of the row height (~1.2 px at body size).
const STROKE: f32 = 0.075;

impl TodoPane {
    /// The pane's cells with each row's checkbox painted at full size.
    /// `aspect` is the frame's `cell_h / cell_w`.
    pub(crate) fn art(&self, cols: u16, rows: u16, aspect: f32) -> (Vec<CellView>, Vec<Paint>) {
        upgrade(self.cells(cols, rows), aspect)
    }
}

/// Replace every mark cell with a painted box: an outline to do, an accent
/// fill under a drawn `✔` once done.
pub(crate) fn upgrade(mut cells: Vec<CellView>, aspect: f32) -> (Vec<CellView>, Vec<Paint>) {
    let page = crew_theme::theme().page_bg;
    let mut paint = Vec::new();
    for c in cells.iter_mut().filter(|c| c.col == BOX_COL + 1) {
        let done = match c.c {
            OPEN => false,
            DONE => true,
            _ => continue,
        };
        let (w, h) = (SIDE * aspect, SIDE);
        let (x, y) = (c.col as f32 + 0.5 - w / 2.0, c.row as f32 + 0.5 - h / 2.0);
        if done {
            paint.push(Paint::solid(x, y, w, h, c.fg));
            c.c = '\u{2714}'; // the heavy tick: a 1 px ✓ vanished in the fill
            c.fg = page;
        } else {
            let (tv, th) = (STROKE, STROKE * aspect);
            paint.push(Paint::solid(x, y, w, tv, c.fg));
            paint.push(Paint::solid(x, y + h - tv, w, tv, c.fg));
            paint.push(Paint::solid(x, y, th, h, c.fg));
            paint.push(Paint::solid(x + w - th, y, th, h, c.fg));
            c.c = ' ';
        }
    }
    (cells, paint)
}

#[cfg(test)]
#[path = "checkbox_tests.rs"]
mod tests;

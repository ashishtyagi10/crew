//! The open nav on a RELIEF theme (neumorphism): ONE raised panel, the
//! whole column moulded from the page like a pane's card, with its sections
//! parted inside it. It was a column of raised tiles for a day, one per
//! section (2026-11-13), and the user took that back (2026-11-14: "all of
//! those item should inside on panel in soft look, instead of separate
//! pill ... a panel and then sections for separation").
//!
//! What parts the sections is read off the cells the nav already draws. A
//! section opens on its rule (`─ TIME ───`,
//! [`crate::boxdraw::section_header`]): no rule is stroked on a relief
//! theme, so the stroke is blanked and a GROOVE is engraved in the gap
//! above the heading, a dark hairline over a light one. A tab's card
//! ([`crate::tabcard`]) is a box in a card, so it is pressed in as a well
//! like every other ([`crate::reliefwell`]). Rows are untouched: every hit
//! path still reads the same [`crate::navlayout`].
use crew_render::{CellView, Paint};

use crate::layout::Rect;
use crate::reliefwell::Box4;

const RULE: char = '\u{2500}';

/// The rows a section opens on: those with a rule at column 1, in order.
pub(crate) fn heads(cells: &[CellView]) -> Vec<u16> {
    let mut v: Vec<u16> = cells
        .iter()
        .filter(|c| c.col == 1 && c.c == RULE)
        .map(|c| c.row)
        .collect();
    v.sort_unstable();
    v
}

/// Where the grooves lie, in rows from the nav's first: one above every
/// section but the first (the panel's own top edge opens that one). It
/// runs through the middle of the gap row over the heading, or along the
/// heading's top edge where a short nav left no gap.
pub(crate) fn grooves(cells: &[CellView]) -> Vec<f32> {
    let inked = |row: u16| cells.iter().any(|c| c.row == row && !c.c.is_whitespace());
    heads(cells)
        .into_iter()
        .skip(1)
        .map(|h| match inked(h - 1) {
            true => f32::from(h),
            false => f32::from(h) - 0.5,
        })
        .collect()
}

/// `cells` with every section's rule blanked. The heading's words stay.
pub(crate) fn unruled(cells: &mut [CellView]) {
    let heads = heads(cells);
    for c in cells.iter_mut() {
        if c.c == RULE && heads.contains(&c.row) {
            c.c = ' ';
        }
    }
}

/// The grooves at `ys` as paint, for a nav `cols` wide with `ch` px rows:
/// the page's dark shadow as a hairline and its light one straight under
/// it, the two a relief card is moulded with, so the line reads as cut
/// into the panel. They span the columns the rules did, 1 to `cols - 2`.
pub(crate) fn groove_paint(ys: &[f32], cols: u16, ch: f32) -> Vec<Paint> {
    let g = crew_theme::glass_style();
    let line = (ch / 16.0).round().max(1.0) / ch.max(1.0);
    let w = f32::from(cols.saturating_sub(2));
    ys.iter()
        .flat_map(|&y| {
            [
                Paint::solid(1.0, y - line, w, line, (0, 0, 0)).at(g.shadow_alpha),
                Paint::solid(1.0, y, w, line, g.highlight).at(g.highlight_alpha),
            ]
        })
        .collect()
}

/// Where a tab card's well lies, for a nav whose cells start at `origin`.
/// Two cards stand on adjacent rows, so each gives `gap` px of its last,
/// emptied row: a rib of the panel between one well and the next.
pub(crate) fn well_rect(b: &Box4, origin: (f32, f32), cw: f32, ch: f32, gap: f32) -> Rect {
    let r = crate::reliefwell::rect_of(b, origin, cw, ch);
    Rect {
        h: (r.h - gap.min(ch)).max(0.0),
        ..r
    }
}

#[cfg(test)]
#[path = "navrelief_tests.rs"]
mod tests;

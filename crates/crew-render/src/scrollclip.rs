//! A pane's content drawn a fraction of a row off the grid, clipped to its
//! card — the renderer's half of pixel-smooth scrolling.
//!
//! The app scrolls in whole rows and hands over the remainder as
//! [`crate::PaneScene::scroll_px`], with one more row of cells under the
//! last so the strip the shift uncovers has the line that belongs there.
//! Text already clips to its scene; the cell backgrounds, rules, cursor and
//! paint are quads that did not, so a shifted pane would have painted its
//! top row over its own frame. They are shifted and cut here, after the pane
//! has pushed them, so nothing that builds them has to know.
use crate::quads::Quad;

/// A scene `h` px tall as laid out and cut: `(rows, text height, clip
/// height)`. Shifted, it lays out one more row, for the line coming in at
/// the bottom, and is cut at its last whole row rather than its rect: the
/// slack under that row shows nothing at rest, so a sliver of the incoming
/// line there would appear and vanish at the ends of every glide.
pub(crate) fn window(h: f32, cell_h: f32, scroll_px: f32) -> (usize, f32, f32) {
    let rows = ((h / cell_h).floor() as usize).max(1);
    match scroll_px != 0.0 {
        true => (rows + 1, h + cell_h, rows as f32 * cell_h),
        false => (rows, h, h),
    }
}

/// Move `quads` down by `dy` and cut them to `top..bottom`, dropping any left
/// with no height. A cut edge loses its rounding: the curve belongs to an
/// edge that is now outside the card.
pub(crate) fn shift_and_clip(quads: &mut Vec<Quad>, dy: f32, top: f32, bottom: f32) {
    if dy == 0.0 {
        return;
    }
    quads.retain_mut(|q| {
        let (y0, y1) = (q.y + dy, q.y + dy + q.h);
        let (c0, c1) = (y0.max(top), y1.min(bottom));
        if c1 <= c0 {
            return false;
        }
        if c0 > y0 {
            q.radii[0] = 0.0;
            q.radii[1] = 0.0;
        }
        if c1 < y1 {
            q.radii[2] = 0.0;
            q.radii[3] = 0.0;
        }
        q.y = c0;
        q.h = c1 - c0;
        true
    });
}

#[cfg(test)]
#[path = "scrollclip_tests.rs"]
mod tests;

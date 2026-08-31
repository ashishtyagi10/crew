//! Heatmap: a grid of cells shaded by magnitude — the shape of *when*.
//!
//! A series says how much and a bar chart says how much per bucket; neither
//! says which hours of which days the work happened in. Seven rows of
//! twenty-four squares does, at a glance, in the space of a paragraph.
//!
//! Shading is by rank against the grid's own peak, not against an absolute
//! scale: a quiet week and a busy one both have to read, and the eye compares
//! within a picture, not between pictures.
use crate::plot::Canvas;

/// A cell's colour, given its value's share of the peak (`0.0..=1.0`).
pub type Shade<'a> = &'a dyn Fn(f32) -> ((u8, u8, u8), f32);

/// Draw `values` (`rows` × `cols` of them, row-major) as a grid filling
/// `(x, y, w, h)` in canvas units, with `gap` units of air between cells.
///
/// An all-zero grid still draws: every cell takes the shade of zero, so an
/// idle week reads as an empty grid rather than as a missing widget.
pub fn draw(
    c: &mut Canvas,
    rect: (f32, f32, f32, f32),
    values: &[u64],
    rows: usize,
    cols: usize,
    gap: f32,
    shade: Shade,
) {
    let (x, y, w, h) = rect;
    if rows == 0 || cols == 0 || w <= 0.0 || h <= 0.0 {
        return;
    }
    let peak = values.iter().copied().max().unwrap_or(0).max(1) as f32;
    let cw = w / cols as f32;
    let ch = h / rows as f32;
    // Cells are square-ish by construction: the caller sizes the rect, and a
    // gap eats the same amount from both axes so the squares stay square.
    let (bw, bh) = ((cw - gap).max(0.02), (ch - gap).max(0.02));
    for r in 0..rows {
        for k in 0..cols {
            let v = values.get(r * cols + k).copied().unwrap_or(0);
            let (color, alpha) = shade(v as f32 / peak);
            if alpha <= 0.0 {
                continue;
            }
            let (bx, by) = (x + k as f32 * cw, y + r as f32 * ch);
            // Rounded corners at this size would cost more than they show;
            // the gap already separates the cells.
            c.rect(bx, by, bw, bh, color, alpha);
        }
    }
}

#[cfg(test)]
#[path = "heatmap_tests.rs"]
mod tests;

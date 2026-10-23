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

/// A used hour's strength, `k` its share of the peak: never under [`FLOOR`].
/// At the old tenth-strength floor the quietest used hour, a whole tile,
/// stood 1.3:1 (light) and 1.5:1 (neon) against the page while an idle
/// hour's dot stood 2.1:1 and 2.8:1, so the eye read the idle dots as the
/// marks and the light use as nothing (UI survey 2026-10-22, J1). `/dash`
/// and `/usage` both shade by this.
pub(crate) fn heat_alpha(k: f32) -> f32 {
    FLOOR + (1.0 - FLOOR) * k.clamp(0.0, 1.0).powf(0.6)
}

/// See [`heat_alpha`].
const FLOOR: f32 = 0.5;

/// An idle cell's dot, as a share of its tile's shorter side.
const DOT: f32 = 0.3;

/// An idle cell's dot is not a reading on the heat scale, so it is not drawn
/// in the scale's colour: it is the border's neutral, the canvas's one mark
/// for "nothing measured". In the scale's zero (a tenth-strength dot a third
/// of a tile wide) it vanished on a dark page, and a day with nothing in it
/// read as a row missing from the grid; at 0.7 it outshone the quietest used
/// hour beside it. Half strength reads (1.6:1 on either Ink Reader page) and
/// stays quieter than any tile.
const IDLE_ALPHA: f32 = 0.5;

/// An idle dot on glass: the glass's own ink at this strength, a lighter
/// frost over whatever desktop is under it. The border's neutral was picked
/// for the page; through the frost over a white desktop it measured 1.1:1
/// on four palettes and a quiet week vanished, and lifted until it read
/// there it stood 7.6:1 over black (glass survey D-L2). A wash of the ink
/// reads 1.9–3:1 over any desktop: plainly a dot, still quieter than a word.
const IDLE_GLASS: f32 = 0.35;

/// An idle dot's colour and alpha on `t`.
pub(crate) fn idle_ink(t: &crew_theme::Theme) -> ((u8, u8, u8), f32) {
    match t.liquid {
        None => (t.border_normal, IDLE_ALPHA),
        Some(_) => (t.ink, IDLE_GLASS),
    }
}

/// A cell's colour, given its value's share of the peak (`0.0..=1.0`).
pub type Shade<'a> = &'a dyn Fn(f32) -> ((u8, u8, u8), f32);

/// Draw `values` (`rows` × `cols` of them, row-major) as a grid filling
/// `(x, y, w, h)` in canvas units, with `gap` units of air between cells.
///
/// An all-zero grid still draws: every cell is a dot in the shade of zero,
/// so an idle week reads as an empty grid rather than as a missing widget.
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
            if v == 0 {
                // An idle hour is a dot at the cell's centre: a quiet week
                // is a calm dot grid, not a slab of empty tiles.
                let d = DOT * bw.min(bh);
                let (idle, alpha) = idle_ink(&crew_theme::theme());
                c.rect(bx + (bw - d) / 2.0, by + (bh - d) / 2.0, d, d, idle, alpha);
                continue;
            }
            // Rounded corners at this size would cost more than they show;
            // the gap already separates the cells.
            c.rect(bx, by, bw, bh, color, alpha);
        }
    }
}

#[cfg(test)]
#[path = "heatmap_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "heatmap_weight_tests.rs"]
mod weight_tests;

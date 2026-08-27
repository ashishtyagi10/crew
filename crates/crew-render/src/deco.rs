//! Turning a cell's [`Deco`] into rectangles.
//!
//! Underlines are drawn as quads rather than glyph decorations: the shaper
//! only knows about glyphs, and a rule that has to line up across a run of
//! cells — a squiggle crossing six columns of a diagnostic — has to be
//! computed from the pane's own pixel grid or it breaks at every cell edge.
//!
//! Every phase here is taken from the *absolute* x, never from the cell's
//! left edge, so a wave or a dot pattern continues across cells instead of
//! restarting inside each one.
use crew_theme::deco::{Deco, DecoLine};

/// `(x, y, w, h)` in pixels.
pub(crate) type Rect = (f32, f32, f32, f32);

/// Rule thickness for a cell of this height, at least one pixel.
pub(crate) fn thickness(cell_h: f32) -> f32 {
    (cell_h * 0.07).round().max(1.0)
}

/// Top edge of the single underline: a gap of one thickness below it, so the
/// rule never touches the cell below.
fn under_y(y: f32, cell_h: f32, t: f32) -> f32 {
    (y + cell_h - 2.0 * t).round()
}

/// A run of `on`-pixel dashes with `off`-pixel gaps, phased on absolute x.
fn dashes(x: f32, y: f32, w: f32, t: f32, on: f32, off: f32) -> Vec<Rect> {
    let period = on + off;
    let mut out = Vec::new();
    let mut px = 0.0_f32;
    while px < w {
        // Where this pixel column sits inside the pattern, from absolute x.
        let phase = (x + px).rem_euclid(period);
        let run = (on - phase).min(w - px);
        if phase < on && run > 0.0 {
            out.push((x + px, y, run, t));
            px += run;
        } else {
            px += 1.0;
        }
    }
    out
}

/// One period of the squiggle per cell, sampled a pixel at a time.
fn curl(x: f32, y: f32, w: f32, cell_w: f32, t: f32) -> Vec<Rect> {
    // Deliberately not a whole fraction of the cell: a period that divides
    // the cell width restarts the wave at every boundary whether or not the
    // phase is absolute, which hides the bug this file exists to avoid.
    let period = (cell_w * 0.66).round().max(4.0);
    let amp = t;
    (0..w.max(1.0) as usize)
        .map(|i| {
            let px = x + i as f32;
            let phase = px.rem_euclid(period) / period * std::f32::consts::TAU;
            (px, (y + amp * phase.sin()).round(), 1.0, t)
        })
        .collect()
}

/// The colour the rules are drawn in: SGR 58's, when the program set one,
/// otherwise the cell's own foreground.
pub(crate) fn color(deco: &Deco, fg: (u8, u8, u8)) -> (u8, u8, u8) {
    deco.color.unwrap_or(fg)
}

/// Every rectangle this decoration draws for one cell.
pub(crate) fn rects(deco: &Deco, x: f32, y: f32, cell_w: f32, cell_h: f32) -> Vec<Rect> {
    if deco.is_blank() {
        return Vec::new();
    }
    let t = thickness(cell_h);
    let uy = under_y(y, cell_h, t);
    let mut out = match deco.line {
        DecoLine::None => Vec::new(),
        DecoLine::Single => vec![(x, uy, cell_w, t)],
        DecoLine::Double => vec![(x, uy - 2.0 * t, cell_w, t), (x, uy, cell_w, t)],
        DecoLine::Curly => curl(x, uy, cell_w, cell_w, t),
        DecoLine::Dotted => dashes(x, uy, cell_w, t, t, t),
        DecoLine::Dashed => dashes(x, uy, cell_w, t, 3.0 * t, 2.0 * t),
    };
    if deco.strike {
        out.push((x, (y + (cell_h - t) * 0.5).round(), cell_w, t));
    }
    out
}

#[cfg(test)]
#[path = "deco_tests.rs"]
mod deco_tests;

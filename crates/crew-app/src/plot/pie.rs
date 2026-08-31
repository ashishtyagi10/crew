//! Pie and donut charts.
//!
//! A proportion is the one thing a bar chart in a cell grid says badly: three
//! bars of 6, 3 and 1 cells are three lengths the eye has to compare, while
//! one ring says *half, a third, a sixth* without arithmetic. Nothing in a
//! terminal font can draw an arc — this is the first widget that could not
//! exist before the [paint layer](crew_render::Paint).
//!
//! Slices start at twelve o'clock and run clockwise, the direction every pie
//! outside a terminal runs, with a hairline gap between neighbours so two
//! adjacent slices of similar colour still read as two.
use std::f32::consts::TAU;

use crate::plot::{sdf, Canvas};

/// Angular gap between slices, in radians — about a degree and a half at the
/// sizes crew draws, which separates without eating a small slice.
const GAP: f32 = 0.026;

/// One wedge: a magnitude and the colour it is drawn in. Zero and negative
/// values draw nothing (a category with no members is not a hairline).
#[derive(Debug, Clone, Copy)]
pub struct Slice {
    pub value: f32,
    pub color: (u8, u8, u8),
}

impl Slice {
    pub fn new(value: f32, color: (u8, u8, u8)) -> Self {
        Self { value, color }
    }
}

/// Draw `slices` as a ring centred at `centre` with outer radius `r_out` and
/// hole radius `r_in` (`0.0` for a full pie), in canvas units.
///
/// With nothing to show — no slices, or every value zero — a dim track ring is
/// drawn instead of nothing at all: an empty crew and a broken widget must not
/// look the same.
pub fn donut(
    c: &mut Canvas,
    centre: (f32, f32),
    r_out: f32,
    r_in: f32,
    slices: &[Slice],
    empty_color: (u8, u8, u8),
) {
    let (cx, cy) = centre;
    let r_in = r_in.clamp(0.0, r_out.max(0.0));
    let bbox = (cx - r_out, cy - r_out, 2.0 * r_out, 2.0 * r_out);
    let total: f32 = slices.iter().map(|s| s.value.max(0.0)).sum();
    if r_out <= 0.0 {
        return;
    }
    if total <= 0.0 {
        c.fill_sdf(bbox, empty_color, 0.5, move |x, y| {
            sdf::sector((x, y), centre, r_out, r_in, 0.0, TAU)
        });
        return;
    }

    // One slice covering everything is a full ring: with a gap it would show a
    // seam that says "two categories" when there is one.
    let drawn = slices.iter().filter(|s| s.value > 0.0).count();
    let gap = if drawn > 1 { GAP } else { 0.0 };
    let mut start = 0.0_f32;
    for s in slices {
        let v = s.value.max(0.0);
        if v <= 0.0 {
            continue;
        }
        let sweep = v / total * TAU;
        let (a0, a1) = (start + gap * 0.5, start + sweep - gap * 0.5);
        start += sweep;
        if a1 <= a0 {
            continue;
        }
        // The sector is described about its own mid-angle, so a slice that
        // crosses twelve o'clock needs no wrap case: `a1` past `TAU` is just
        // a mid-angle past `TAU`.
        c.fill_sdf(bbox, s.color, 1.0, move |x, y| {
            sdf::sector((x, y), centre, r_out, r_in, a0, a1)
        });
    }
}

/// A filled circle — the legend swatch beside a slice's label, and the mark a
/// scatter or a series head is drawn with.
pub fn dot(c: &mut Canvas, centre: (f32, f32), r: f32, color: (u8, u8, u8), alpha: f32) {
    let (cx, cy) = centre;
    c.fill_sdf(
        (cx - r, cy - r, 2.0 * r, 2.0 * r),
        color,
        alpha,
        move |x, y| sdf::disc((x, y), centre, r),
    );
}

#[cfg(test)]
#[path = "pie_tests.rs"]
mod tests;

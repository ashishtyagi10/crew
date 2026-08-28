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
mod tests {
    use super::{donut, dot, Slice};
    use crate::plot::Canvas;
    use std::f32::consts::PI;

    /// Ink of one colour, in square units — a slice's *area*, which is what a
    /// pie is read by.
    fn area_of(c: &Canvas, color: (u8, u8, u8)) -> f32 {
        c.paint()
            .iter()
            .filter(|p| {
                let d = |a: u8, b: u8| (a as i32 - b as i32).abs();
                d(p.color.0, color.0) + d(p.color.1, color.1) + d(p.color.2, color.2) < 24
            })
            .map(|p| p.w * p.h * c.row_units() * p.alpha)
            .sum()
    }

    fn canvas() -> Canvas {
        Canvas::with_sub(12, 6, 2.0, 8)
    }

    const RED: (u8, u8, u8) = (255, 0, 0);
    const BLUE: (u8, u8, u8) = (0, 0, 255);
    const GREY: (u8, u8, u8) = (128, 128, 128);

    #[test]
    fn a_donut_is_a_ring_with_a_hole_the_size_it_was_asked_for() {
        let mut c = canvas();
        donut(&mut c, (6.0, 6.0), 4.0, 2.0, &[Slice::new(1.0, RED)], GREY);
        let ink = area_of(&c, RED);
        let want = PI * (4.0 * 4.0 - 2.0 * 2.0);
        assert!(
            (ink - want).abs() / want < 0.03,
            "ring area {ink}, expected {want}"
        );
    }

    #[test]
    fn slice_areas_are_proportional_to_their_values() {
        let mut c = canvas();
        donut(
            &mut c,
            (6.0, 6.0),
            4.0,
            0.0,
            &[Slice::new(3.0, RED), Slice::new(1.0, BLUE)],
            GREY,
        );
        let (r, b) = (area_of(&c, RED), area_of(&c, BLUE));
        assert!(
            (r / (r + b) - 0.75).abs() < 0.03,
            "3:1 split came out {r}:{b}"
        );
    }

    #[test]
    fn the_first_slice_starts_at_twelve_oclock_and_runs_clockwise() {
        let mut c = canvas();
        donut(
            &mut c,
            (6.0, 6.0),
            4.0,
            0.0,
            &[Slice::new(1.0, RED), Slice::new(3.0, BLUE)],
            GREY,
        );
        // A quarter turn clockwise from noon is the upper-RIGHT quadrant.
        let quadrant = |xs: std::ops::Range<f32>, ys: std::ops::Range<f32>, col: (u8, u8, u8)| {
            c.paint()
                .iter()
                .filter(|p| {
                    p.color == col && xs.contains(&p.x) && ys.contains(&(p.y * c.row_units()))
                })
                .count()
        };
        assert!(quadrant(6.0..10.0, 2.0..6.0, RED) > 0, "red is top-right");
        assert_eq!(quadrant(2.0..6.0, 2.0..6.0, RED), 0, "and not top-left");
    }

    #[test]
    fn neighbouring_slices_are_separated_but_a_single_slice_is_whole() {
        let one = {
            let mut c = canvas();
            donut(&mut c, (6.0, 6.0), 4.0, 2.0, &[Slice::new(1.0, RED)], GREY);
            area_of(&c, RED)
        };
        let split = {
            let mut c = canvas();
            donut(
                &mut c,
                (6.0, 6.0),
                4.0,
                2.0,
                &[Slice::new(0.5, RED), Slice::new(0.5, RED)],
                GREY,
            );
            area_of(&c, RED)
        };
        // The same total value, drawn as two slices, loses exactly the gaps —
        // a few percent, not a seam through a single-category ring.
        assert!(
            split < one * 0.995,
            "two slices show a gap: {split} < {one}"
        );
        assert!(
            split > one * 0.95,
            "the gap is a hairline: {split} vs {one}"
        );
    }

    #[test]
    fn an_empty_series_draws_a_track_not_a_blank() {
        let mut c = canvas();
        donut(&mut c, (6.0, 6.0), 4.0, 2.0, &[], GREY);
        assert!(area_of(&c, GREY) > 10.0, "the empty ring is drawn");
        // Every value zero reads the same way: nothing running, not nothing
        // rendered.
        let mut c2 = canvas();
        donut(&mut c2, (6.0, 6.0), 4.0, 2.0, &[Slice::new(0.0, RED)], GREY);
        assert!(area_of(&c2, GREY) > 10.0);
        assert_eq!(area_of(&c2, RED), 0.0, "a zero slice is not a hairline");
    }

    #[test]
    fn a_dot_is_round_and_where_it_was_put() {
        let mut c = canvas();
        dot(&mut c, (3.0, 3.0), 1.0, RED, 1.0);
        let ink = area_of(&c, RED);
        assert!((ink - PI).abs() / PI < 0.05, "disc area {ink} vs {PI}");
        for p in c.paint() {
            assert!(p.x >= 1.9 && p.x <= 4.1, "dot stayed put: {p:?}");
        }
    }
}

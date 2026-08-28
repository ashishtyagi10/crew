//! An instrument dial: a needle on a ticked face, read the way a clock is.
//!
//! The ring gauge ([`crate::plot::gauge`]) answers "how full" with a swept
//! band, which is honest but wordless — a band has no scale on it, so a
//! reading can only be compared against the ring's own emptiness, and the
//! number has to live in the hole where it crowds the stroke from both sides.
//!
//! A dial says the same thing the way every physical instrument does: a fixed
//! scale with ticks, and a hand pointing at a place on it. The eye reads a
//! *position* against marks it has already learnt, so half is half without a
//! number, and the marks the hand has passed light up so length says it too.
//! The face is open at the bottom — the 270° arc a speedometer uses, from
//! half past seven round to half past four — and that gap is where the digits
//! go, out of the geometry instead of inside it.
//!
//! Everything scales off one radius, so the same dial draws at a nav's six
//! columns and at a dashboard card's twenty.
use std::f32::consts::TAU;

use crate::plot::{sdf, Canvas};

/// How far the scale runs: two thirds of the way round. A speedometer's 270°
/// is the other common choice and it was the first one tried here — it ends
/// the scale at half past four, which is exactly where the digits sit, so the
/// last tick and the last digit fight over the same few pixels. Stopping at
/// four o'clock opens the bottom third of the face and the number moves into
/// it cleanly.
pub const SPAN: f32 = 2.0 / 3.0 * TAU;
/// …and where it begins: eight o'clock, measured clockwise from noon.
/// Negative because the scale straddles noon — which is the point of the
/// layout, since noon is where an instrument puts the middle of its range.
pub const START: f32 = -0.5 * SPAN;
/// Ticks on the scale, ends included — one every tenth.
const TICKS: usize = 11;

/// One dial. Colours come from the caller so a tier's hue, the theme's track
/// and the card's own plate are all decided in one place, beside the reading.
#[derive(Debug, Clone, Copy)]
pub struct Dial {
    pub centre: (f32, f32),
    /// The bezel's radius, in canvas units. Everything else is a fraction of
    /// it.
    pub r: f32,
    pub frac: f32,
    /// Needle, hub, and the ticks the needle has passed.
    pub color: (u8, u8, u8),
    /// Bezel, and the major ticks the needle has not reached.
    pub track: (u8, u8, u8),
    /// The minor ticks it has not reached — one rank quieter than `track`.
    ///
    /// A second colour rather than `track` at a lower alpha, because alpha is
    /// where a scale's contrast goes to die: a tick blended halfway into the
    /// page reads at half the ratio its colour was chosen for, and on the
    /// light pages in this set that is the difference between a scale and a
    /// blank face. Both are laid down opaque and both are the caller's to
    /// measure.
    pub track_dim: (u8, u8, u8),
    /// The face itself: a colour and the alpha to lay it down at, or `None`
    /// for a dial that is only its scale and its hand. A plate gives the
    /// ticks something to sit on; on a grainy page it has to be very faint or
    /// it reads as a smudge, which is why the alpha is the caller's to pick.
    pub plate: Option<((u8, u8, u8), f32)>,
}

/// The angle a reading points at.
pub fn angle_of(frac: f32) -> f32 {
    START + frac.clamp(0.0, 1.0) * SPAN
}

/// Draw `d`.
pub fn draw(c: &mut Canvas, d: Dial) {
    let Dial {
        centre,
        r,
        frac,
        color,
        track,
        track_dim,
        plate,
    } = d;
    if r <= 0.0 {
        return;
    }
    let frac = frac.clamp(0.0, 1.0);
    // One canvas pixel is the floor on every thin feature: below it a mark
    // only gets dimmer, never thinner, so a tick on a small dial fades out
    // instead of flickering in and out between sizes.
    let px = c.px();
    let bbox = (centre.0 - r, centre.1 - r, 2.0 * r, 2.0 * r);

    // The plate. Faint: it is a ground for the ticks, not a disc.
    if let Some((col, alpha)) = plate {
        c.fill_sdf(bbox, col, alpha, move |x, y| sdf::disc((x, y), centre, r));
    }
    // The bezel — an arc over the scale, not a closed ring. Closing it would
    // put a stroke straight through the digits sitting in the gap, and an
    // instrument's bezel ends where its scale does anyway.
    let bez = (0.035 * r).max(px * 0.5);
    let pad = TAU * 0.015;
    c.fill_sdf(bbox, track_dim, 1.0, move |x, y| {
        sdf::arc(
            (x, y),
            centre,
            r - bez,
            bez,
            START - pad,
            START + SPAN + pad,
        )
    });

    // The scale. A tick the needle has passed takes the reading's colour, so
    // the answer is said in position *and* in how much of the scale is lit —
    // the second reading is the one that survives a glance.
    let tw = (0.045 * r).max(px * 0.5);
    for i in 0..TICKS {
        let t = i as f32 / (TICKS - 1) as f32;
        let major = i % 5 == 0;
        let inner = if major { 0.70 } else { 0.80 };
        let a = angle_of(t);
        let p0 = sdf::polar(centre, r * 0.90, a);
        let p1 = sdf::polar(centre, r * inner, a);
        // A hair of tolerance: the tick under the needle lights when the
        // needle reaches it, not a hair before.
        let lit = t <= frac + 1e-4;
        let col = if lit {
            color
        } else if major {
            track
        } else {
            track_dim
        };
        let w = if major { tw } else { tw * 0.7 };
        // A tick's own box, not the face's: a distance field answers
        // everywhere it is asked, and asking over the whole face eleven times
        // is most of what drawing a dial used to cost.
        let tb = sdf::bounds(&[p0, p1], w + px);
        c.fill_sdf(tb, col, 1.0, move |x, y| sdf::capsule((x, y), p0, p1, w));
    }

    // The hand, as two tapers off the hub rather than one bar across it: a
    // single cone from the counterweight to the tip is thickest *behind* the
    // pivot and reads as a lump with a whisker, which is what the first
    // version of this looked like. Widest at the pivot, a point at the tip,
    // a stub the other way for balance.
    let a = angle_of(frac);
    let hub = (0.085 * r).max(px * 0.6);
    let tip = sdf::polar(centre, r * 0.74, a);
    let tail = sdf::polar(centre, r * 0.15, a + TAU * 0.5);
    let point = (0.02 * r).max(px * 0.5);
    let hb = sdf::bounds(&[centre, tip, tail], hub + px);
    c.fill_sdf(hb, color, 1.0, move |x, y| {
        sdf::cone((x, y), centre, tip, hub, point).min(sdf::cone(
            (x, y),
            centre,
            tail,
            hub,
            hub * 0.55,
        ))
    });
    // The pivot last, over the hand's own root: a hand is pinned to the face,
    // it does not grow out of it.
    let pb = sdf::bounds(&[centre], hub * 1.15 + px);
    c.fill_sdf(pb, color, 1.0, move |x, y| {
        sdf::disc((x, y), centre, hub * 1.15)
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILL: (u8, u8, u8) = (0, 255, 0);
    const TRACK: (u8, u8, u8) = (150, 150, 150);
    const DIM: (u8, u8, u8) = (70, 70, 70);
    const PLATE: Option<((u8, u8, u8), f32)> = Some(((20, 20, 30), 0.5));

    fn dial(frac: f32) -> Canvas {
        let mut c = Canvas::with_sub(9, 3, 2.0, 16);
        draw(
            &mut c,
            Dial {
                centre: (4.5, 3.0),
                r: 2.8,
                frac,
                color: FILL,
                track: TRACK,
                track_dim: DIM,
                plate: PLATE,
            },
        );
        c
    }

    /// Two colours within rounding of each other: the canvas stores
    /// premultiplied and un-premultiplies at emit, and every mark here is
    /// drawn over a translucent plate.
    fn near(a: (u8, u8, u8), b: (u8, u8, u8)) -> bool {
        let d = |x: u8, y: u8| (x as i32 - y as i32).abs();
        d(a.0, b.0) + d(a.1, b.1) + d(a.2, b.2) < 40
    }

    /// Ink of one colour inside a box, in square units.
    fn ink_in(c: &Canvas, color: (u8, u8, u8), b: (f32, f32, f32, f32)) -> f32 {
        c.paint()
            .iter()
            .filter(|p| near(p.color, color))
            .map(|p| {
                let w = (p.x + p.w).min(b.0 + b.2) - p.x.max(b.0);
                let h = ((p.y + p.h).min(b.1 + b.3) - p.y.max(b.1)) * c.row_units();
                w.max(0.0) * h.max(0.0) * p.alpha
            })
            .sum()
    }

    fn ink(c: &Canvas, color: (u8, u8, u8)) -> f32 {
        ink_in(c, color, (0.0, 0.0, 100.0, 100.0))
    }

    /// The whole point of a dial: the hand moves, and where it points is the
    /// reading. Empty points down-left, half points straight up, full points
    /// down-right.
    #[test]
    fn the_needle_points_where_the_reading_is() {
        // A box on the hand's shaft, inside the tick ring — measuring out at
        // the tip would count the lit ticks the hand has swept past, and a
        // full dial lights every tick on the face.
        let hand_box = |frac: f32| {
            let (x, y) = sdf::polar((4.5, 3.0), 2.8 * 0.45, angle_of(frac));
            (x - 0.4, (y - 0.4) / 2.0, 0.8, 0.8 / 2.0)
        };
        for frac in [0.0, 0.5, 1.0] {
            let c = dial(frac);
            let here = ink_in(&c, FILL, hand_box(frac));
            assert!(here > 0.05, "the hand reaches {frac}: {here}");
            for other in [0.0, 0.5, 1.0] {
                if (other - frac).abs() < 1e-6 {
                    continue;
                }
                let there = ink_in(&c, FILL, hand_box(other));
                assert!(
                    there < here * 0.3,
                    "and not to {other} as well ({there} vs {here})"
                );
            }
        }
    }

    /// The scale is fixed, so an empty dial is still a dial: the ticks and
    /// the bezel are there to read the hand against.
    #[test]
    fn an_empty_dial_still_shows_the_scale_it_would_fill() {
        let c = dial(0.0);
        let scale = ink(&c, TRACK) + ink(&c, DIM);
        assert!(scale > 1.0, "bezel and ticks: {scale}");
    }

    /// …and the ticks the hand has passed carry the reading too, so it is
    /// said by length as well as by angle.
    #[test]
    fn the_lit_ticks_follow_the_reading() {
        // Counted at the ticks' outer ends, past where the hand reaches: ink
        // summed over the whole face would be measuring the needle.
        let lit = |frac: f32| {
            let c = dial(frac);
            (0..TICKS)
                .filter(|i| {
                    let t = *i as f32 / (TICKS - 1) as f32;
                    let (x, y) = sdf::polar((4.5, 3.0), 2.8 * 0.87, angle_of(t));
                    ink_in(&c, FILL, (x - 0.25, (y - 0.25) / 2.0, 0.5, 0.5 / 2.0)) > 0.01
                })
                .count()
        };
        assert_eq!(lit(0.0), 1, "empty lights only the tick at zero");
        assert_eq!(lit(0.5), 6, "half lights half of them");
        assert_eq!(lit(1.0), TICKS, "full lights the scale");
    }

    /// The bottom of the face is deliberately open — it is where the digits
    /// go. Nothing on the scale, bezel included, may be drawn across it.
    #[test]
    fn the_scale_leaves_the_bottom_of_the_face_clear() {
        let c = dial(0.5);
        // Under the hub, from inside the tick ring out to just short of the
        // bezel: two columns wide, the space two digits ask for.
        let (r, cy) = (2.8, 3.0);
        let below = (4.5 - 1.0, (cy + 0.55 * r) / 2.0, 2.0, 0.45 * r / 2.0);
        let marks = ink_in(&c, TRACK, below) + ink_in(&c, DIM, below) + ink_in(&c, FILL, below);
        assert!(marks < 0.05, "the digits' gap is clear: {marks}");
    }

    /// A dial with no radius is not a crash and not a dot.
    #[test]
    fn a_dial_with_no_room_draws_nothing() {
        let mut c = Canvas::new(9, 3, 2.0);
        draw(
            &mut c,
            Dial {
                centre: (4.5, 3.0),
                r: 0.0,
                frac: 0.5,
                color: FILL,
                track: TRACK,
                track_dim: DIM,
                plate: PLATE,
            },
        );
        assert!(c.paint().is_empty());
    }
}

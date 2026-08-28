//! Arc gauges: a reading drawn as a swept ring rather than a filled bar.
//!
//! A bar answers "how full" by length, which needs a track beside it to
//! compare against and a number to be sure. A ring answers it by *angle* —
//! quarter, half, three-quarters are read without measuring — and it puts the
//! number in the middle of its own shape instead of at the end of a row.
//!
//! The sweep starts at twelve o'clock and runs clockwise, ends rounded, over a
//! dim full ring so an empty gauge still shows where full would be.
use std::f32::consts::TAU;

use crate::plot::pie;
use crate::plot::Canvas;

/// Draw one gauge: a `track`-coloured ring with `frac` of it swept in `color`.
/// `frac` outside `0..=1` is clamped — a gauge that reads past full is a bug
/// in the sampler, not a shape.
pub fn arc(
    c: &mut Canvas,
    centre: (f32, f32),
    r_out: f32,
    r_in: f32,
    frac: f32,
    color: (u8, u8, u8),
    track: (u8, u8, u8),
) {
    let (cx, cy) = centre;
    let r_in = r_in.clamp(0.0, r_out.max(0.0));
    if r_out <= 0.0 {
        return;
    }
    let bbox = (cx - r_out, cy - r_out, 2.0 * r_out, 2.0 * r_out);
    let in_ring = move |x: f32, y: f32| {
        let d2 = (x - cx).powi(2) + (y - cy).powi(2);
        d2 <= r_out * r_out && d2 >= r_in * r_in
    };
    // The whole ring, dim: an empty gauge has to show the shape it would fill.
    c.fill(bbox, track, 0.55, in_ring);

    let frac = frac.clamp(0.0, 1.0);
    if frac <= 0.0 {
        return;
    }
    let sweep = frac * TAU;
    c.fill(bbox, color, 1.0, move |x, y| {
        if !in_ring(x, y) {
            return false;
        }
        let a = (x - cx).atan2(-(y - cy));
        let a = if a < 0.0 { a + TAU } else { a };
        a <= sweep
    });
    // Rounded ends. The cap at the sweep's head is what tells a 3% reading
    // from an empty gauge at sidebar sizes, where 3% of a circle is thinner
    // than the ring is wide.
    let cap_r = (r_out - r_in) * 0.5;
    let cap = |a: f32| {
        let r = (r_out + r_in) * 0.5;
        (cx + r * a.sin(), cy - r * a.cos())
    };
    pie::dot(c, cap(0.0), cap_r, color, 1.0);
    pie::dot(c, cap(sweep), cap_r, color, 1.0);
}

#[cfg(test)]
mod tests {
    use super::arc;
    use crate::plot::Canvas;

    const FILL: (u8, u8, u8) = (0, 255, 0);
    const TRACK: (u8, u8, u8) = (60, 60, 60);

    /// Two colours within rounding of each other: the canvas stores
    /// premultiplied and un-premultiplies at emit, so a translucent fill comes
    /// back a shade off what went in.
    fn near(a: (u8, u8, u8), b: (u8, u8, u8)) -> bool {
        let d = |x: u8, y: u8| (x as i32 - y as i32).abs();
        d(a.0, b.0) + d(a.1, b.1) + d(a.2, b.2) < 24
    }

    /// Area of one colour in square units.
    fn area(c: &Canvas, color: (u8, u8, u8)) -> f32 {
        c.paint()
            .iter()
            .filter(|p| near(p.color, color))
            .map(|p| p.w * p.h * c.row_units() * p.alpha)
            .sum()
    }

    fn gauge(frac: f32) -> Canvas {
        let mut c = Canvas::with_sub(8, 4, 2.0, 8);
        arc(&mut c, (4.0, 4.0), 3.0, 1.8, frac, FILL, TRACK);
        c
    }

    #[test]
    fn the_swept_area_follows_the_reading() {
        let quarter = area(&gauge(0.25), FILL);
        let half = area(&gauge(0.5), FILL);
        // Caps add a little at both ends, so this is a ratio check, not an
        // equality: half a ring is about twice a quarter.
        assert!(
            (half / quarter - 2.0).abs() < 0.25,
            "quarter {quarter}, half {half}"
        );
    }

    #[test]
    fn an_empty_gauge_still_shows_where_full_would_be() {
        let c = gauge(0.0);
        assert!(area(&c, TRACK) > 5.0, "the track ring is drawn");
        assert_eq!(area(&c, FILL), 0.0, "and nothing is swept");
    }

    #[test]
    fn a_full_gauge_covers_the_track() {
        let full = gauge(1.0);
        assert!(
            area(&full, TRACK) < area(&gauge(0.0), TRACK) * 0.05,
            "a full sweep leaves no track showing"
        );
    }

    #[test]
    fn a_tiny_reading_is_still_visible() {
        // 2% of a circle is thinner than the ring is wide: without the round
        // cap it would rasterize to nothing and read as zero.
        let c = gauge(0.02);
        assert!(area(&c, FILL) > 0.4, "2% draws a visible mark");
    }

    #[test]
    fn the_sweep_starts_at_noon_and_runs_clockwise() {
        let c = gauge(0.25);
        // Ink is clipped to each half rather than sorted by which half a
        // rectangle starts in — the run merge produces rectangles that
        // straddle the centre line, and counting those whole reads as a
        // quarter of the ring on the wrong side.
        let ink_in = |x0: f32, x1: f32| -> f32 {
            c.paint()
                .iter()
                .filter(|p| near(p.color, FILL))
                .map(|p| {
                    let w = (p.x + p.w).min(x1) - p.x.max(x0);
                    w.max(0.0) * p.h * c.row_units() * p.alpha
                })
                .sum()
        };
        // Left of the start cap's own radius: the cap straddles noon by
        // design, everything beyond it would be a sweep running the wrong way.
        let cap_r = (3.0 - 1.8) * 0.5;
        let left = ink_in(0.0, 4.0 - cap_r);
        let right = ink_in(4.0, 8.0);
        assert!(right > 2.0, "the quarter is drawn: {right}");
        assert!(left < 0.1, "and nothing swept left: {left}");
    }

    #[test]
    fn a_reading_past_full_is_clamped_not_wrapped() {
        assert!((area(&gauge(1.5), FILL) - area(&gauge(1.0), FILL)).abs() < 0.2);
    }
}

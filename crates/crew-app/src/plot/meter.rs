//! Capsule meters: a proportion drawn as a rounded bar.
//!
//! The dithered `▓`/`░` gauges these replace could only be eight cells long
//! and eight steps deep, so a budget crossing 6% moved nothing at all and a
//! meter at 12% and one at 24% drew the same first cell. A drawn capsule is
//! continuous — the fill lands where the number says, to a fraction of a cell.
use crate::plot::{sdf, Canvas};

/// How tall the capsule is inside its row, as a fraction of the row.
const HEIGHT: f32 = 0.30;

/// Draw a meter filling `frac` of `(x, y, w)` — one text row tall, centred in
/// it. `shade` gives the fill's colour along its length (`0.0..=1.0`), so a
/// meter can walk a gradient; `track` is the unfilled remainder, which is
/// always drawn: a meter's length is half of what it says.
#[allow(clippy::too_many_arguments)] // a meter is a rect, a reading, and two colours
pub fn capsule(
    c: &mut Canvas,
    x: f32,
    y: f32,
    w: f32,
    row_h: f32,
    frac: f32,
    shade: impl Fn(f32) -> (u8, u8, u8),
    track: (u8, u8, u8),
) {
    let h = (row_h * HEIGHT).max(0.12);
    let top = y + (row_h - h) * 0.5;
    let r = h * 0.5;
    if w <= 0.0 {
        return;
    }
    // The track: the full length, so the eye has something to read the fill
    // against even at 0%.
    rounded(c, x, top, w, h, r, |_| track, 0.55);

    let frac = frac.clamp(0.0, 1.0);
    if frac <= 0.0 {
        return;
    }
    // A fill shorter than its own end caps would draw as a lozenge floating
    // in the track; below that it is drawn as a dot at the left end, which is
    // what "barely started" looks like.
    let fw = (w * frac).max(h.min(w));
    rounded(c, x, top, fw, h, r, |t| shade(t * frac.max(0.001)), 1.0);
}

/// A rounded rectangle: the pill shape both the track and the fill are.
#[allow(clippy::too_many_arguments)] // a rect, a corner radius, a shade, an alpha
fn rounded(
    c: &mut Canvas,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    r: f32,
    shade: impl Fn(f32) -> (u8, u8, u8),
    alpha: f32,
) {
    c.fill_sdf_shaded(
        (x, y, w, h),
        move |px, py| sdf::round_box((px, py), x, y, w, h, r),
        move |px, _| {
            let t = if w > 0.0 {
                ((px - x) / w).clamp(0.0, 1.0)
            } else {
                0.0
            };
            (shade(t), alpha)
        },
    );
}

#[cfg(test)]
mod tests {
    use super::capsule;
    use crate::plot::Canvas;

    const FILL: (u8, u8, u8) = (0, 220, 120);
    const TRACK: (u8, u8, u8) = (70, 70, 70);

    fn meter(frac: f32) -> Canvas {
        let mut c = Canvas::with_sub(10, 1, 2.0, 8);
        capsule(&mut c, 1.0, 0.0, 8.0, 2.0, frac, |_| FILL, TRACK);
        c
    }

    fn ink(c: &Canvas, color: (u8, u8, u8)) -> f32 {
        let near = |a: (u8, u8, u8)| {
            let d = |x: u8, y: u8| (x as i32 - y as i32).abs();
            d(a.0, color.0) + d(a.1, color.1) + d(a.2, color.2) < 30
        };
        c.paint()
            .iter()
            .filter(|p| near(p.color))
            .map(|p| p.w * p.h * c.row_units() * p.alpha)
            .sum()
    }

    /// The right edge of the fill, in canvas units.
    fn fill_end(c: &Canvas) -> f32 {
        let near = |a: (u8, u8, u8)| {
            let d = |x: u8, y: u8| (x as i32 - y as i32).abs();
            d(a.0, FILL.0) + d(a.1, FILL.1) + d(a.2, FILL.2) < 30
        };
        c.paint()
            .iter()
            .filter(|p| near(p.color))
            .map(|p| p.x + p.w)
            .fold(0.0f32, f32::max)
    }

    #[test]
    fn the_fill_lands_where_the_number_says() {
        // Half of an 8-unit meter starting at x=1 ends at 5, within a pixel.
        assert!(
            (fill_end(&meter(0.5)) - 5.0).abs() < 0.3,
            "{}",
            fill_end(&meter(0.5))
        );
        assert!((fill_end(&meter(1.0)) - 9.0).abs() < 0.3);
    }

    #[test]
    fn a_percent_the_glyph_meter_could_not_show_moves_it() {
        // The eight-cell dithered gauge drew 12% and 24% identically. These
        // must differ by about an eighth of the meter's length.
        let (a, b) = (fill_end(&meter(0.12)), fill_end(&meter(0.24)));
        assert!(b - a > 0.6, "12% ends at {a}, 24% at {b}");
    }

    #[test]
    fn an_empty_meter_still_shows_its_length() {
        let c = meter(0.0);
        assert!(ink(&c, TRACK) > 1.0, "the track is drawn");
        assert_eq!(ink(&c, FILL), 0.0);
    }

    #[test]
    fn a_barely_started_meter_draws_a_mark_not_nothing() {
        let c = meter(0.005);
        assert!(ink(&c, FILL) > 0.05, "1/200th still marks the left end");
        assert!(fill_end(&c) < 2.5, "and stays at the left end");
    }

    #[test]
    fn the_fill_can_walk_a_gradient_along_its_length() {
        let mut c = Canvas::with_sub(10, 1, 2.0, 8);
        capsule(
            &mut c,
            1.0,
            0.0,
            8.0,
            2.0,
            1.0,
            |t| (((1.0 - t) * 255.0) as u8, (t * 255.0) as u8, 0),
            TRACK,
        );
        let reds = c.paint().iter().filter(|p| p.color.0 > 200).count();
        let greens = c.paint().iter().filter(|p| p.color.1 > 200).count();
        assert!(reds > 0 && greens > 0, "both ends of the ramp are drawn");
    }
}

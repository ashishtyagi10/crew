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

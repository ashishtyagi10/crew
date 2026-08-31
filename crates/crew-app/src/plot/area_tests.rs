use super::{draw, value_at};
use crate::plot::Canvas;

#[test]
fn the_curve_passes_through_every_sample() {
    let s = [0.0, 1.0, 0.25, 0.8];
    for (i, want) in s.iter().enumerate() {
        let t = i as f32 / (s.len() - 1) as f32;
        assert!((value_at(&s, t) - want).abs() < 1e-4, "sample {i}");
    }
}

#[test]
fn interpolation_never_overshoots_the_data() {
    // A step the naive spline would ring on: the curve must stay inside
    // the two samples it is between, or the chart shows a spike the
    // machine never had.
    let s = [0.0, 0.0, 1.0, 1.0, 0.0, 0.0];
    for k in 0..=200 {
        let t = k as f32 / 200.0;
        let v = value_at(&s, t);
        assert!((-1e-5..=1.0 + 1e-5).contains(&v), "t={t} v={v}");
    }
}

#[test]
fn a_flat_series_fills_its_share_of_the_box() {
    // Half-height series → the fill covers about half the chart, and the
    // *shape* carries the reading (the glyph sparkline it replaced had
    // eight levels total, so 0.30 and 0.37 drew the same row).
    let mut c = Canvas::new(20, 2, 2.0);
    let (w, h) = c.size();
    draw(&mut c, (0.0, 0.0, w, h), &[0.5; 8], (0, 200, 255));
    let ink: f32 = c
        .paint()
        .iter()
        .map(|p| p.w * p.h * c.row_units() * p.alpha)
        .sum();
    let box_area = w * h;
    // Fill fades from 0.38 to 0.04 over the covered half, plus the stroke.
    assert!(
        (0.06..0.20).contains(&(ink / box_area)),
        "covered fraction {}",
        ink / box_area
    );
}

#[test]
fn a_higher_series_paints_more_than_a_lower_one() {
    let ink = |v: f32| {
        let mut c = Canvas::new(20, 2, 2.0);
        let (w, h) = c.size();
        draw(&mut c, (0.0, 0.0, w, h), &[v; 8], (0, 200, 255));
        c.paint().iter().map(|p| p.w * p.h * p.alpha).sum::<f32>()
    };
    let (low, mid, high) = (ink(0.1), ink(0.5), ink(0.95));
    assert!(low < mid && mid < high, "{low} < {mid} < {high}");
}

#[test]
fn an_empty_series_draws_nothing() {
    let mut c = Canvas::new(10, 2, 2.0);
    draw(&mut c, (0.0, 0.0, 10.0, 4.0), &[], (0, 0, 0));
    assert!(c.paint().is_empty());
}

#[test]
fn the_newest_sample_sits_at_the_right_edge() {
    // Zero everywhere but the last reading: the ink must be on the right.
    let mut c = Canvas::new(20, 2, 2.0);
    let (w, h) = c.size();
    let mut s = [0.0f32; 10];
    s[9] = 1.0;
    draw(&mut c, (0.0, 0.0, w, h), &s, (0, 200, 255));
    let top: Vec<_> = c
        .paint()
        .into_iter()
        .filter(|p| p.y < h / c.row_units() * 0.25)
        .collect();
    assert!(!top.is_empty(), "the peak reached the top of the box");
    assert!(
        top.iter().all(|p| p.x > 15.0),
        "the peak is at the right edge: {:?}",
        top.iter().map(|p| p.x).collect::<Vec<_>>()
    );
}

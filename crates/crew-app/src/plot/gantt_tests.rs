use super::{draw, rule, Span};
use crate::plot::Canvas;

const RUN: (u8, u8, u8) = (0, 220, 120);
const TRACK: (u8, u8, u8) = (80, 80, 80);

fn span(a: u64, b: u64) -> Option<Span> {
    Some(Span {
        start_ms: a,
        end_ms: b,
        color: RUN,
    })
}

fn chart(spans: &[Option<Span>], t0: u64, t1: u64) -> Canvas {
    let mut c = Canvas::with_sub(20, spans.len() as u16, 2.0, 8);
    draw(&mut c, (0.0, 0.0, 20.0), 2.0, spans, t0, t1, TRACK);
    c
}

/// The bar's ink, and where it starts and ends, ignoring the track.
fn bar(c: &Canvas, row: usize) -> Option<(f32, f32)> {
    let near = |a: (u8, u8, u8)| {
        let d = |x: u8, y: u8| (x as i32 - y as i32).abs();
        d(a.0, RUN.0) + d(a.1, RUN.1) + d(a.2, RUN.2) < 24
    };
    let rows = (row as f32)..(row as f32 + 1.0);
    let xs: Vec<(f32, f32)> = c
        .paint()
        .iter()
        .filter(|p| near(p.color) && rows.contains(&p.y))
        .map(|p| (p.x, p.x + p.w))
        .collect();
    let lo = xs.iter().map(|p| p.0).fold(f32::MAX, f32::min);
    let hi = xs.iter().map(|p| p.1).fold(0.0f32, f32::max);
    (hi > 0.0).then_some((lo, hi))
}

#[test]
fn a_span_lands_on_the_axis_where_its_times_say() {
    // Second half of a 0..1000 axis, on a 20-unit-wide chart.
    let c = chart(&[span(500, 1000)], 0, 1000);
    let (lo, hi) = bar(&c, 0).expect("the bar is drawn");
    assert!((lo - 10.0).abs() < 0.4, "starts halfway: {lo}");
    assert!((hi - 20.0).abs() < 0.4, "ends at the right edge: {hi}");
}

#[test]
fn tasks_that_ran_at_once_show_as_bars_that_overlap() {
    // The question a task list cannot answer.
    let parallel = chart(&[span(0, 900), span(50, 950), span(80, 1000)], 0, 1000);
    let serial = chart(&[span(0, 300), span(330, 660), span(700, 1000)], 0, 1000);
    let overlap = |c: &Canvas| -> bool {
        let a = bar(c, 0).unwrap();
        let b = bar(c, 1).unwrap();
        a.1 > b.0 + 0.5
    };
    assert!(overlap(&parallel), "parallel bars overlap in time");
    assert!(!overlap(&serial), "serial bars do not");
}

#[test]
fn an_instant_task_still_draws_a_mark() {
    let c = chart(&[span(400, 400)], 0, 1000);
    let (lo, hi) = bar(&c, 0).expect("a zero-length span is still drawn");
    assert!(hi - lo > 0.2, "with a visible width: {lo}..{hi}");
}

#[test]
fn a_row_with_no_span_still_has_its_lane() {
    let c = chart(&[None, span(0, 500)], 0, 1000);
    assert!(bar(&c, 0).is_none(), "nothing ran on row 0");
    // …but the row is not blank: the lanes are what line the chart up
    // with the list beside it.
    let track = c
        .paint()
        .into_iter()
        .filter(|p| p.y < 1.0 && p.color != RUN)
        .count();
    assert!(track > 0, "the empty row keeps its track");
}

#[test]
fn a_span_running_past_the_axis_is_clipped_not_wrapped() {
    let c = chart(&[span(0, 99_000)], 0, 1000);
    let (_, hi) = bar(&c, 0).unwrap();
    assert!(hi <= 20.0 + 1e-3, "clipped to the chart: {hi}");
}

#[test]
fn a_rule_marks_one_instant_across_every_row() {
    let mut c = Canvas::with_sub(20, 3, 2.0, 8);
    rule(&mut c, 10.0, 0.0, 6.0, (255, 0, 0), 0.8);
    let p = c.paint();
    assert!(!p.is_empty());
    for r in &p {
        assert!((r.x - 10.0).abs() < 0.3, "on the instant: {r:?}");
    }
    let bottom = p.iter().map(|r| r.y + r.h).fold(0.0f32, f32::max);
    assert!(bottom > 2.5, "and down all three rows: {bottom}");
}

use super::{paint, ROWS};
use crate::spark::History;

const DOWN: (u8, u8, u8) = (0, 160, 255);
const UP: (u8, u8, u8) = (0, 255, 120);

fn hist(vals: &[u64]) -> History {
    let mut h = History::new(64);
    for &v in vals {
        h.push(v);
    }
    h
}

/// Ink of one colour above / below the block's centre line, in rows.
fn split(p: &[crew_render::Paint], row0: u16, color: (u8, u8, u8)) -> (f32, f32) {
    let mid = f32::from(row0) + f32::from(ROWS) / 2.0;
    let near = |a: (u8, u8, u8)| {
        let d = |x: u8, y: u8| (x as i32 - y as i32).abs();
        d(a.0, color.0) + d(a.1, color.1) + d(a.2, color.2) < 24
    };
    let mut above = 0.0;
    let mut below = 0.0;
    for r in p.iter().filter(|r| near(r.color)) {
        let ink = r.w * r.h * r.alpha;
        if r.y + r.h <= mid + 1e-3 {
            above += ink;
        } else if r.y >= mid - 1e-3 {
            below += ink;
        }
    }
    (above, below)
}

#[test]
fn down_grows_up_and_up_grows_down() {
    let _g = crate::app::theme_test_guard();
    let p = paint(&hist(&[900; 8]), &hist(&[900; 8]), 24, 4, 2.0, DOWN, UP);
    let (d_above, d_below) = split(&p, 4, DOWN);
    let (u_above, u_below) = split(&p, 4, UP);
    assert!(
        d_above > d_below * 8.0,
        "down is above: {d_above}/{d_below}"
    );
    assert!(u_below > u_above * 8.0, "up is below: {u_below}/{u_above}");
}

#[test]
fn both_halves_share_one_scale() {
    let _g = crate::app::theme_test_guard();
    // Ten times the download of the upload. Measured by how far each
    // curve reaches from the centre line, not by ink: the stroke and the
    // head dot cost the same on both halves whatever the reading is.
    // Scaled independently the two would reach equally far and say the
    // traffic was balanced.
    // Both well above the chart's floor, so the axis is theirs.
    let p = paint(
        &hist(&[4_000_000; 8]),
        &hist(&[400_000; 8]),
        24,
        0,
        2.0,
        DOWN,
        UP,
    );
    let mid = f32::from(ROWS) / 2.0;
    let near = |a: (u8, u8, u8), b: (u8, u8, u8)| {
        let d = |x: u8, y: u8| (x as i32 - y as i32).abs();
        d(a.0, b.0) + d(a.1, b.1) + d(a.2, b.2) < 24
    };
    let up_reach = p
        .iter()
        .filter(|r| near(r.color, UP))
        .map(|r| r.y + r.h - mid)
        .fold(0.0f32, f32::max);
    let down_reach = p
        .iter()
        .filter(|r| near(r.color, DOWN))
        .map(|r| mid - r.y)
        .fold(0.0f32, f32::max);
    assert!(
        down_reach > up_reach * 3.0,
        "down reaches {down_reach}, up {up_reach}"
    );
}

#[test]
fn an_idle_network_draws_its_axis_and_nothing_else() {
    let _g = crate::app::theme_test_guard();
    let axis = crew_theme::theme().border_normal;
    let p = paint(&hist(&[0; 8]), &hist(&[0; 8]), 24, 0, 2.0, DOWN, UP);
    // Named by colour, not by "something was drawn": the old assertion
    // passed on the two curves' strokes while the axis itself was thinner
    // than a canvas pixel and never rendered at all.
    let rule: Vec<_> = p.iter().filter(|r| r.color == axis).collect();
    assert!(!rule.is_empty(), "the centre line is drawn: {p:?}");
    assert!(
        rule.iter().any(|r| r.w >= 19.0),
        "and spans the chart: {rule:?}"
    );
    let mid = f32::from(ROWS) / 2.0;
    assert!(
        p.iter().all(|r| (r.y - mid).abs() < 0.35),
        "and everything drawn sits on it: {:?}",
        p.iter().map(|r| r.y).collect::<Vec<_>>()
    );
    // The direction curves have faded out: an idle link is an axis, not a
    // full-width saturated band.
    let loud = p
        .iter()
        .filter(|r| r.color != axis && r.alpha > 0.3)
        .count();
    assert_eq!(loud, 0, "idle traffic still drew: {p:?}");
}

#[test]
fn an_idle_link_does_not_fill_the_chart() {
    let _g = crate::app::theme_test_guard();
    // A few hundred bytes a second of background chatter. Scaled to its
    // own peak this drew a full-height band and read as a saturated link.
    let p = paint(&hist(&[300; 8]), &hist(&[120; 8]), 24, 0, 2.0, DOWN, UP);
    let mid = f32::from(ROWS) / 2.0;
    let reach = p
        .iter()
        .filter(|r| r.color == DOWN)
        .map(|r| mid - r.y)
        .fold(0.0f32, f32::max);
    // Not zero — the curve keeps its stroke and its head dot at the
    // baseline — but a fraction of what a real transfer draws.
    assert!(reach < 0.3, "an idle link drew {reach} rows of chart");
    // …and a real transfer still fills it.
    let busy = paint(
        &hist(&[9_000_000; 8]),
        &hist(&[10; 8]),
        24,
        0,
        2.0,
        DOWN,
        UP,
    );
    let loud = busy
        .iter()
        .filter(|r| r.color == DOWN)
        .map(|r| mid - r.y)
        .fold(0.0f32, f32::max);
    assert!(loud > 0.8, "a saturated link drew only {loud} rows");
    assert!(loud > reach * 3.0, "idle {reach} vs busy {loud}");
}

#[test]
fn with_no_history_at_all_nothing_is_drawn() {
    let _g = crate::app::theme_test_guard();
    let empty = History::new(8);
    assert!(paint(&empty, &empty, 24, 0, 2.0, DOWN, UP).is_empty());
}

#[test]
fn the_chart_stays_in_its_rows_and_indent() {
    let _g = crate::app::theme_test_guard();
    let p = paint(
        &hist(&[500, 900, 100]),
        &hist(&[10, 700, 20]),
        24,
        6,
        2.0,
        DOWN,
        UP,
    );
    assert!(!p.is_empty());
    for r in &p {
        assert!(r.x >= 3.0, "indented under the legend: {r:?}");
        assert!(r.x + r.w <= 23.0 + 1e-3, "and inside the card: {r:?}");
        assert!(
            r.y >= 6.0 && r.y + r.h <= 6.0 + f32::from(ROWS) + 1e-3,
            "{r:?}"
        );
    }
}

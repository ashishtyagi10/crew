//! The NET section's twin chart: bytes down drawn upward from a centre line,
//! bytes up drawn downward from it.
//!
//! The single throughput sparkline it replaces summed both directions, so a
//! machine pulling a container image and one pushing a backup drew the same
//! line — and the one thing you want from a network chart is *which way*.
//!
//! Both halves share one scale (the larger direction's peak), because two
//! independently-scaled halves would draw a trickle of uploads the same height
//! as a flood of downloads and quietly lie about the balance.
use crew_render::Paint;

use crate::plot::{area, Canvas};

/// Rows the chart occupies: one per direction.
pub const ROWS: u16 = 2;

/// The smallest peak the axis will scale to, in bytes per second. Anything
/// quieter draws small rather than full.
const FLOOR: u64 = 64 * 1024;

/// Samples of each history the chart at `cols` wide draws from.
fn span(cols: u16) -> usize {
    cols.saturating_sub(4) as usize * 2
}

/// The scale both halves are drawn against, in bytes per second — the larger
/// direction's peak, never below [`FLOOR`]. The NET rule writes it down: the
/// chart's ceiling moves with the traffic, and a shape with a moving ceiling
/// and no ceiling written down is a shape you cannot read a value off.
pub fn ceiling(rx: &crate::spark::History, tx: &crate::spark::History, cols: u16) -> u64 {
    let s = span(cols);
    rx.peak(s).max(tx.peak(s)).max(FLOOR)
}

/// Draw the twin chart across `cols` starting at `row0`, indented under the
/// section legend like the rates above it. `rx`/`tx` are the two histories.
pub fn paint(
    rx: &crate::spark::History,
    tx: &crate::spark::History,
    cols: u16,
    row0: u16,
    aspect: f32,
    down: (u8, u8, u8),
    up: (u8, u8, u8),
) -> Vec<Paint> {
    let (col0, width) = (3u16, cols.saturating_sub(4));
    if width == 0 || (rx.is_empty() && tx.is_empty()) {
        return Vec::new();
    }
    let span = span(cols);
    // One scale for both halves — see the module note — with a floor under
    // it. Auto-scaling to the window's own peak makes an idle machine's
    // background chatter (a few hundred bytes a second) fill the chart, which
    // reads as a saturated link. Below the floor the chart stays small,
    // because below the floor nothing is happening. Same derivation the NET
    // rule's key reads, so the number and the shape cannot disagree.
    let peak = ceiling(rx, tx, cols);
    let norm = |h: &crate::spark::History| -> Vec<f32> {
        h.tail(span)
            .into_iter()
            .map(|v| (v as f32 / peak as f32).clamp(0.0, 1.0))
            .collect()
    };

    let mut c = Canvas::new(width, ROWS, aspect);
    let (w, h) = c.size();
    let half = h / 2.0;
    // How loud each direction is against the floor. A flat curve still costs
    // a full-strength stroke plus a head dot, and at zero both strokes land
    // on the centre line together — which drew a solid, near-opaque band the
    // full width of the nav and read as a saturated link on an idle machine.
    // Fading a direction out as it approaches silence leaves the axis alone
    // on the row, which is what "nothing is moving" should look like.
    let voice = |h: &crate::spark::History| -> f32 {
        (h.peak(span) as f32 / FLOOR as f32).clamp(0.0, 1.0).sqrt()
    };
    let (rx_v, tx_v) = (voice(rx), voice(tx));
    // Down grows up out of the centre line; up grows down from it. `area`
    // always fills toward the bottom of the box it is given, so the upper half
    // is drawn into its own box and the lower half into a box of its own with
    // the series flipped… which would put its baseline at the wrong edge.
    // Instead the lower half is drawn upside down and mirrored back.
    let style = area::Style::anchored();
    let mut upper = Canvas::new(width, ROWS, aspect);
    area::draw_styled(&mut upper, (0.0, 0.0, w, half), &norm(rx), down, style);
    for p in upper.paint() {
        c.rect(
            p.x,
            p.y * aspect,
            p.w,
            p.h * aspect,
            p.color,
            p.alpha * rx_v,
        );
    }
    let mut lower = Canvas::new(width, ROWS, aspect);
    area::draw_styled(&mut lower, (0.0, 0.0, w, half), &norm(tx), up, style);
    for p in lower.paint() {
        // Mirror about the centre line: a rectangle at distance d below the
        // top of its box lands at distance d below the centre.
        let y_units = p.y * aspect;
        let hgt = p.h * aspect;
        c.rect(
            p.x,
            half + (half - y_units - hgt),
            p.w,
            hgt,
            p.color,
            p.alpha * tx_v,
        );
    }
    // The centre line the two halves grow from, so an idle network still has
    // an axis rather than a blank gap. A hairline, not a `rect`: at 0.06 units
    // it was thinner than a canvas pixel and fell between the coverage
    // samples, so what looked like an axis on an idle link was really the two
    // flat curves' own strokes lying on top of each other.
    c.hairline(0.0, half, w, crew_theme::theme().border_normal, 0.7);

    c.paint()
        .into_iter()
        .map(|p| p.shifted(f32::from(col0), f32::from(row0)))
        .collect()
}

#[cfg(test)]
mod tests {
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
}

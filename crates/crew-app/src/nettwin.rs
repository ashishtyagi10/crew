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
#[path = "nettwin_tests.rs"]
mod tests;

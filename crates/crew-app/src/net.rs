//! Sidebar network section: a `NET` divider above down/up byte rates, over a
//! twin chart ([`crate::nettwin`]) that draws the two directions apart —
//! down growing up out of a centre line, up growing down from it.
use crew_render::CellView;

use crate::palette::accent;
/// Blue-cyan for the throughput chart (distinct from the green CPU chart).
/// The throughput trace's blue, lightened or darkened until the page it sits
/// on can show it. As the flat constant `(120, 200, 255)` it read at 1.6 on
/// every light theme — a sparkline nobody could see.
pub fn spark() -> (u8, u8, u8) {
    crew_theme::readable::spark(crew_theme::theme())
}

/// Format a per-second byte rate compactly, e.g. `0 B/s`, `12 KB/s`, `3.4 MB/s`.
pub fn rate(bytes: u64) -> String {
    let b = bytes as f64;
    if b < 1024.0 {
        format!("{bytes} B/s")
    } else if b < 1024.0 * 1024.0 {
        format!("{:.0} KB/s", b / 1024.0)
    } else {
        format!("{:.1} MB/s", b / (1024.0 * 1024.0))
    }
}

/// The colour bytes *up* are drawn in — the accent, against the down
/// direction's blue, so the two halves of the twin chart are told apart by
/// hue as well as by which side of the line they are on.
pub fn up_color() -> (u8, u8, u8) {
    accent()
}

/// Render the network section: a `NET` rule on row 0 and the `↓ rx  ↑ tx`
/// rates on row 1. The chart under them is drawn, not spelled — see
/// [`crate::nettwin`], reached from the sidebar's paint layer.
///
/// The rates are values, so a nav too narrow for both gives up a whole one
/// (the quieter direction) rather than half of one: `↑ 0 B` is not a smaller
/// reading than `↑ 0 B/s`, it is a different unit.
pub fn net_cells(rx: u64, tx: u64, ceiling: u64, cols: u16) -> Vec<CellView> {
    if cols < 10 {
        return Vec::new();
    }
    let t = crew_theme::theme();
    // The twin chart under these rates shares one moving scale between its two
    // halves; the rule writes that scale down, so the shape can be read as a
    // value and not only as a shape.
    let mut out = crate::boxdraw::section_header_key(
        "NET",
        &format!("peak {}", rate(ceiling)),
        cols,
        t.border_normal,
        accent(),
        t.dim,
        t.page_bg,
    );
    let (down, up) = (rate(rx), rate(tx));
    // Longest first: both with air, both tight, then whichever direction is
    // carrying more — on a link doing nothing that is `↓`, which is the one
    // you would ask about anyway.
    let busier = if tx > rx { &up } else { &down };
    let arrow = if tx > rx { '↑' } else { '↓' };
    let ladder = [
        format!("↓ {down}  ↑ {up}"),
        format!("↓ {down} ↑ {up}"),
        format!("{arrow} {busier}"),
    ];
    let refs: Vec<&str> = ladder.iter().map(String::as_str).collect();
    let shown = crate::navtext::fit(&refs, cols);
    // Both directions and room to spare: the up rate goes to the right edge
    // instead of trailing the down rate, so a wide nav reads as two readings
    // on one row rather than one short run and a lot of nothing. The two are
    // opposite directions — putting them at opposite ends says so.
    let split = crate::navtext::budget(cols) >= shown.chars().count() + SPREAD_AIR;
    if split && shown.starts_with('↓') && shown.contains('↑') {
        let (d, u) = (format!("↓ {down}"), format!("↑ {up}"));
        crate::navtext::put(&mut out, &d, 1, cols, t.ink);
        let right = cols.saturating_sub(1);
        let at = right.saturating_sub(crate::chatwidth::str_w(&u) as u16);
        crate::navtext::put_at(&mut out, &u, at, 1, right, t.ink);
        return out;
    }
    crate::navtext::put(&mut out, shown, 1, cols, t.ink);
    out
}

/// Extra columns beyond the tight form before the two rates move apart. Below
/// this they are close enough that spreading them looks like a mistake.
const SPREAD_AIR: usize = 6;

#[cfg(test)]
#[path = "net_tests.rs"]
mod tests;

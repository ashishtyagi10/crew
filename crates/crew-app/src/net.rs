//! Sidebar network section: a `NET` divider above down/up byte rates, over a
//! twin chart ([`crate::nettwin`]) that draws the two directions apart —
//! down growing up out of a centre line, up growing down from it.
use crew_render::CellView;

use crate::boxdraw::section_header;

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
pub fn net_cells(rx: u64, tx: u64, cols: u16) -> Vec<CellView> {
    if cols < 10 {
        return Vec::new();
    }
    let t = crew_theme::theme();
    let mut out = section_header("NET", cols, t.border_normal, accent(), t.page_bg);
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
    crate::navtext::put(&mut out, crate::navtext::fit(&refs, cols), 1, cols, t.ink);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_units() {
        assert_eq!(rate(0), "0 B/s");
        assert_eq!(rate(500), "500 B/s");
        assert_eq!(rate(2048), "2 KB/s");
        assert_eq!(rate(3_500_000), "3.3 MB/s");
    }

    #[test]
    fn net_section_has_rule_and_both_rates() {
        // The colours are derived from the live theme now, so two reads of
        // the process global must not straddle another test switching it.
        let _g = crate::app::theme_test_guard();
        let cells = net_cells(2048, 1024, 24);
        assert!(cells.iter().any(|c| c.c == '─' && c.row == 0));
        assert!(!cells.iter().any(|c| c.c == '╭'));
        // both rates share row 1
        assert!(cells.iter().any(|c| c.c == '↓' && c.row == 1));
        assert!(cells.iter().any(|c| c.c == '↑' && c.row == 1));
        // and the chart rows below carry no glyphs at all: the twin chart is
        // drawn on the paint layer, and a leftover block ramp here would show
        // through it.
        assert!(!cells.iter().any(|c| c.row >= 2));
    }

    /// A nav too narrow for both rates shows the busier direction whole, not
    /// both of them cut. `↑ 0 B` is a different unit, not a smaller reading.
    #[test]
    fn a_narrow_nav_drops_a_direction_rather_than_its_unit() {
        let _g = crate::app::theme_test_guard();
        let row = |cols| -> String {
            let mut v: Vec<_> = net_cells(4_000_000, 900, cols)
                .into_iter()
                .filter(|c| c.row == 1)
                .collect();
            v.sort_by_key(|c| c.col);
            v.iter().map(|c| c.c).collect()
        };
        assert_eq!(row(28), "↓ 3.8 MB/s  ↑ 900 B/s");
        assert_eq!(row(24), "↓ 3.8 MB/s ↑ 900 B/s");
        // Down is carrying more, so down is what survives — whole.
        assert_eq!(row(18), "↓ 3.8 MB/s");
        assert!(!row(18).contains('…'), "a value is dropped, never clipped");
    }

    /// …and when it is the upload that is busy, the upload is what survives.
    #[test]
    fn the_direction_that_survives_is_the_one_carrying_more() {
        let _g = crate::app::theme_test_guard();
        let mut v: Vec<_> = net_cells(900, 4_000_000, 18)
            .into_iter()
            .filter(|c| c.row == 1)
            .collect();
        v.sort_by_key(|c| c.col);
        let row: String = v.iter().map(|c| c.c).collect();
        assert_eq!(row, "↑ 3.8 MB/s");
    }
}

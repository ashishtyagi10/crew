//! Indeterminate progress bar: a bright window of heavy rule glyphs that sweeps
//! back and forth along a pane's bottom border while the pane is busy (a swarm
//! planning/running, an agent chat awaiting a reply). It conveys "working,
//! unknown duration" the way a barber-pole / marquee does — the idiomatic
//! indeterminate signal, since ratatui's `Gauge`/`LineGauge` only show a known
//! ratio. Pure function of `(cols, now_ms)`; the caller paints it onto the frame.
use crate::anim;

/// Milliseconds for one full left→right→left sweep.
const PERIOD_MS: u64 = 1400;
/// Dim colour at the trailing edges of the sweep window.
const EDGE: (u8, u8, u8) = (40, 40, 48);

/// The lit columns of the sweep for a card `cols` wide at time `now_ms`, as
/// `(col, fg)` pairs on the interior span `1..=cols-2` (clear of the corners).
/// The window is brightest at its centre and fades to [`EDGE`] at its edges.
/// Empty when the card is too narrow to host a meaningful bar.
pub fn sweep(cols: u16, now_ms: u64) -> Vec<(u16, (u8, u8, u8))> {
    if cols < 8 {
        return Vec::new();
    }
    let interior = cols - 2; // usable columns 1..=cols-2 (between the corners)
    let w = (interior / 4).clamp(3, interior);
    let travel = interior - w; // range the window's left edge can slide over
    let left = 1 + (anim::tri(now_ms, PERIOD_MS) * travel as f32).round() as u16;
    let base = crate::palette::accent();
    let half = (w as f32 / 2.0).max(1.0);
    (0..w)
        .map(|i| {
            // 0 at the window centre → 1 at its edges; brightest in the middle.
            let d = (i as f32 - (w as f32 - 1.0) / 2.0).abs() / half;
            (left + i, anim::lerp_rgb(base, EDGE, d))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn narrow_cards_have_no_bar() {
        assert!(sweep(7, 0).is_empty());
    }

    #[test]
    fn sweep_stays_within_the_interior() {
        let cols = 40;
        for t in (0..PERIOD_MS).step_by(37) {
            let cells = sweep(cols, t);
            assert!(!cells.is_empty());
            assert!(cells.iter().all(|(c, _)| *c >= 1 && *c <= cols - 2));
        }
    }

    #[test]
    fn window_centre_is_brighter_than_edges() {
        let cells = sweep(40, 0);
        let accent = crate::palette::accent();
        let mid = cells[cells.len() / 2].1;
        let edge = cells[0].1;
        // centre matches (or nears) the accent; the edge is dimmer.
        let lum = |c: (u8, u8, u8)| c.0 as u32 + c.1 as u32 + c.2 as u32;
        assert!(lum(mid) > lum(edge));
        assert!(lum(mid) >= lum(accent) - 3 * 3);
    }

    #[test]
    fn sweep_moves_over_time() {
        let a: Vec<u16> = sweep(40, 0).iter().map(|(c, _)| *c).collect();
        let b: Vec<u16> = sweep(40, PERIOD_MS / 4).iter().map(|(c, _)| *c).collect();
        assert_ne!(a, b);
    }
}

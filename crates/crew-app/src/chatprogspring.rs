//! The progress bar's fill, sweeping rather than jumping.
//!
//! `chatprog` draws the swarm's determinate bar; its fill used to snap from
//! `3/7` of the width to `4/7` the frame a task settled, which reads as a
//! repaint rather than as progress. The fill now follows the run's own
//! [`crate::readout::Counter`] — the same 420 ms ease-out sweep the footer's
//! numbers use — from the fraction it was showing to the new one, and the
//! three leading cells glow while it moves: the bar colour lifted toward the
//! page ink by a raised cosine, brightest at the edge. Off is the old jump.
//!
//! The sweep is on the FRACTION, not the cell count, so a bar that changes
//! width (the words it underlines grow a digit) retargets nothing. The fill
//! is floored from the fraction, so a partly settled run never shows a full
//! bar — the invariant `chatprog` has always kept.
use crate::chatswarm::SwarmStatus;
use crate::shimmer::Color;

/// Leading cells that glow while the fill moves.
pub(crate) const GLOW_CELLS: u16 = 3;

/// Cells of a `bar_w`-wide bar lit at `now`, on the way to `done/total`.
pub(crate) fn filled(s: &SwarmStatus, done: usize, total: usize, bar_w: u16, now: u64) -> u16 {
    let frac = s.fill.tick(done as f64 / total.max(1) as f64, now);
    // The epsilon keeps `1/3 × 15` at 5 rather than 4.999…; the fraction is
    // below 1 while the run is live, so the floor stays short of `bar_w`.
    ((frac * f64::from(bar_w) + 1e-9).floor() as u16).min(bar_w)
}

/// Whether the fill still has frames to draw.
pub(crate) fn live(s: &SwarmStatus, now: u64) -> bool {
    s.fill.live(now)
}

/// The colour of filled cell `i` (0 = the bar's first cell) of `filled`:
/// `bar` at rest; while `live`, the last [`GLOW_CELLS`] cells lift toward
/// `ink` by a raised cosine — 1 at the leading edge, 0 three cells back —
/// floored against `page` at the mark floor.
pub(crate) fn glow(i: u16, filled: u16, live: bool, bar: Color, ink: Color, page: Color) -> Color {
    let back = filled.saturating_sub(1).saturating_sub(i);
    if !live || i >= filled || back >= GLOW_CELLS {
        return bar;
    }
    let w = 0.5 * (1.0 + (std::f32::consts::PI * back as f32 / GLOW_CELLS as f32).cos());
    let want = crate::anim::lerp_rgb(bar, ink, w);
    crew_theme::readable::against(want, page, crew_theme::readable::MARK_FLOOR)
}

#[cfg(test)]
#[path = "chatprogspring_tests.rs"]
mod tests;

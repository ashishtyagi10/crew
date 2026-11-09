//! The indeterminate progress comet on a card's bottom border (OSC 9;4 with
//! no percent, a busy swarm or chat). Its own file because the still frame
//! Motion off asks for is a second drawing, not a parameter of the first.
use crate::plot::{sdf, Canvas};

/// One full bounce of the indeterminate sweep, in ms — the same clock the
/// glyph sweep used, so nothing about the motion changed but its smoothness.
pub(crate) const SWEEP_MS: u64 = 1400;

/// A comet across `inner` columns at `y`, `weight` tall: a block of the
/// same width the glyph sweep used, shaded so its LEADING edge is brightest
/// and it fades away behind — a block of constant brightness cannot say
/// which way it is going, and half of a bounce is spent going the other way.
///
/// With Motion off it is a level segment held at the centre: still, but
/// still saying "working, amount unknown". A tail on a comet that does not
/// move would claim a heading it does not have.
pub(crate) fn comet(c: &mut Canvas, inner: f32, y: f32, weight: f32, fg: (u8, u8, u8), now: u64) {
    let w = (inner / 5.0).max(1.0);
    let travel = (inner - w).max(0.0);
    let phase = crate::anim::sweep(now, SWEEP_MS, 0.5);
    let x0 = 1.0 + phase * travel;
    // Which way it is heading, from the wave a moment later.
    let ahead = crate::anim::sweep(now + SWEEP_MS / 64, SWEEP_MS, 0.5);
    let heading = match crate::attention::steady() {
        true => None,
        false => Some(ahead >= phase),
    };
    c.fill_sdf_shaded(
        (x0, y, w, weight),
        move |px, py| sdf::round_box((px, py), x0, y, w, weight, weight * 0.5),
        move |px, _| {
            let t = ((px - x0) / w).clamp(0.0, 1.0);
            let lead = match heading {
                None => return (fg, 0.95),
                Some(true) => t,
                Some(false) => 1.0 - t,
            };
            (fg, lead.powf(1.6) * 0.95)
        },
    );
}

#[cfg(test)]
#[path = "cardcomet_tests.rs"]
mod tests;

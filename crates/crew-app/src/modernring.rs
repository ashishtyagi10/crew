//! The MODERN family's gradient light-ring (goal 2026-08-10): on the
//! Gemini/Codex-look themes the focused frame stops being a single-colour
//! stroke and becomes a corner-to-corner gradient between the theme's two
//! `ModernStyle` poles — the bloom chain then lifts it into a soft halo.
//! While the pane is streaming the gradient drifts slowly along the frame
//! (one full colour cycle per `drift_ms`); an idle frame never drifts, so
//! idle frames stay byte-identical (the same static-frame determinism
//! contract the CRT trace keeps). Gaining focus reuses the ignition
//! timeline: the whole ring starts lifted toward white and decays to the
//! resting gradient. The white corner nodes of the CRT trace are deliberately
//! absent — a gradient ring with hot rivets reads as a tube, not an app.
use crew_render::CellView;

use crate::panecard::is_frame_glyph;

/// How far the ring starts toward white when focus ignites it (decays to 0
/// over the ignition timeline).
const IGNITE_LIFT: f32 = 0.4;

/// The pole the ignition lifts toward — over-driven white, as in the CRT
/// trace, so the flash reads in plain luminance on any gradient.
const HOT_POLE: (u8, u8, u8) = (255, 255, 255);

/// Recolour the focused frame's stroke cells with the modern gradient ring.
/// Only cells still wearing the plain `border_focused` colour are touched —
/// the legend, focus brackets and status glyphs ride the same border rows and
/// keep their own colours (the CRT trace's contract). No-op on themes without
/// a `ModernStyle`.
pub(crate) fn ring(v: &mut [CellView], cols: u16, rows: u16, busy: bool, ignite_t: f32, now: u64) {
    let t = crew_theme::theme();
    let Some(style) = t.modern else { return };
    let base = t.border_focused;
    // Drift rides the busy redraw cadence and follows the Motion gate; an
    // idle pane's phase is exactly zero, so the resting gradient is a pure
    // function of cell position and idle frames stay byte-identical.
    let phase = if busy && crate::motion::level() != crate::motion::MotionLevel::Off {
        (now % style.drift_ms) as f32 / style.drift_ms as f32
    } else {
        0.0
    };
    let lift = (1.0 - ignite_t) * IGNITE_LIFT;
    let cmax = f32::from(cols.saturating_sub(1).max(1));
    let rmax = f32::from(rows.saturating_sub(1).max(1));
    for c in v.iter_mut() {
        if c.fg != base || !is_frame_glyph(c.c) {
            continue;
        }
        c.fg = ring_color(
            style.pole_a,
            style.pole_b,
            0.5 * (f32::from(c.col) / cmax + f32::from(c.row) / rmax),
            phase,
            lift,
        );
    }
}

/// One stroke cell's colour: `d` is the cell's diagonal position across the
/// frame (0 at the top-left corner, 1 at the bottom-right), `phase` the drift
/// position (0 at rest), `lift` the ignition's white lift. Pure so it is
/// testable, and exact at rest: `phase = 0, lift = 0` maps the top-left
/// corner to `pole_a` and the bottom-right to `pole_b` bit-for-bit (the
/// cosine hits 1 and −1 exactly, and `lerp_rgb` returns its endpoints).
fn ring_color(
    pole_a: (u8, u8, u8),
    pole_b: (u8, u8, u8),
    d: f32,
    phase: f32,
    lift: f32,
) -> (u8, u8, u8) {
    let mix = 0.5 - 0.5 * (std::f32::consts::PI * d + std::f32::consts::TAU * phase).cos();
    let rgb = crate::anim::lerp_rgb(pole_a, pole_b, mix);
    if lift <= 0.0 {
        return rgb;
    }
    crate::anim::lerp_rgb(rgb, HOT_POLE, lift)
}

#[cfg(test)]
#[path = "modernring_tests.rs"]
mod tests;

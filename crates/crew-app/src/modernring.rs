//! The gradient stroke every card in crew now wears.
//!
//! It started (goal 2026-08-10) as the MODERN family's light-ring: on two
//! themes the FOCUSED pane frame stopped being a single-colour stroke and
//! became a corner-to-corner gradient between the theme's two `ModernStyle`
//! poles, which the bloom chain then lifted into a soft halo. v0.18.25 gave
//! every theme its own poles. This module is the third step: the gradient
//! stops being a property of *the focused pane* and becomes a property of
//! *a card*, so the sidebar, the input bar, the command menu, the toasts,
//! the minimized thumbnails and every unfocused pane carry it too.
//!
//! Two strengths, because a canvas where every frame is equally colourful has
//! no focus at all:
//!
//! * [`ring`] — the focused pane. Full poles, and it moves: while the pane is
//!   streaming the gradient drifts one colour cycle per `drift_ms`, and
//!   gaining focus fires the ignition (the whole ring starts lifted toward
//!   white and decays to the resting gradient).
//! * [`quiet`] — everything else. The same gradient, but re-lit to the
//!   stroke's OWN luminance before it is mixed in: hue travels around the
//!   card, brightness does not move. That is what keeps the hierarchy — the
//!   focused frame is still the brightest thing on the canvas, it is just no
//!   longer the only coloured one. A quiet stroke never drifts and never
//!   ignites, so idle frames stay byte-identical (the same static-frame
//!   determinism contract the CRT trace keeps).
//!
//! The white corner nodes of the CRT trace are deliberately absent from both
//! — a gradient ring with hot rivets reads as a tube, not an app.
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
    let (pole_a, pole_b) = live_poles(&style);
    paint(v, cols, rows, base, |d| {
        ring_color(pole_a, pole_b, d, phase, lift)
    });
}

/// How far a quiet stroke moves from its flat border colour toward the
/// luminance-matched gradient. Below ~0.4 the hue reads as a rendering
/// artefact rather than a choice; at 1.0 an unfocused card is as saturated as
/// the focused one and the grid loses its centre.
const QUIET_MIX: f32 = 0.6;

/// Tint an UNFOCUSED card's stroke with the theme gradient, at the stroke's
/// own brightness.
///
/// `base` is the flat colour the card was drawn in; only cells still wearing
/// it are touched, so legends, status glyphs and focus brackets riding the
/// same border rows keep their own colours (the ring's contract). Static by
/// construction — no phase, no ignition — so a canvas full of quiet cards
/// still repaints to the same bytes every time. No-op on a theme without a
/// `ModernStyle`.
pub(crate) fn quiet(v: &mut [CellView], cols: u16, rows: u16, base: (u8, u8, u8)) {
    let Some(style) = crew_theme::theme().modern else {
        return;
    };
    let (pole_a, pole_b) = live_poles(&style);
    paint(v, cols, rows, base, |d| {
        let g = ring_color(pole_a, pole_b, d, 0.0, 0.0);
        crate::anim::lerp_rgb(base, at_luma_of(g, base), QUIET_MIX)
    });
}

/// The theme gradient sampled on a straight line: `t = 0` is `pole_a`,
/// `t = 1` is `pole_b`, and both endpoints are exact. `None` on a theme
/// without a `ModernStyle`, so callers can fall back to their flat colour.
///
/// This is the gradient WITHOUT the ring's seamless cosine loop, for surfaces
/// that run edge to edge instead of around a perimeter — the footer's meters
/// are the first. A loop would put the same colour at both ends of a bar,
/// which is exactly wrong for something read left to right.
pub(crate) fn pole_mix(t: f32) -> Option<(u8, u8, u8)> {
    let style = crew_theme::theme().modern?;
    let (pole_a, pole_b) = live_poles(&style);
    Some(crate::anim::lerp_rgb(pole_a, pole_b, t.clamp(0.0, 1.0)))
}

/// The two poles every gradient surface in crew is drawn between: the
/// theme's own, wearing whatever hue offset the app published this frame
/// (crew-theme's `poleshift`). One accessor, so the page's wash and the
/// cards' strokes can never be a frame apart on the same colour — a canvas
/// where the background and the frames disagreed about the gradient would
/// read as a bug, not as depth.
///
/// At rest the offset is zero and this returns `style`'s own bytes, which is
/// what keeps the quiet stroke's static-frame contract.
fn live_poles(style: &crew_theme::ModernStyle) -> crew_theme::poleshift::Poles {
    crew_theme::poleshift::poles().unwrap_or((style.pole_a, style.pole_b))
}

/// [`crate::boxdraw::titled_card`] with the quiet gradient already on its
/// stroke: the card constructor for every panel that is not a pane — the
/// sidebar sections, the welcome and update cards, the command menu, the
/// paste prompt, the composer, the input bar, the toasts, the minimized
/// thumbnails. Panes build their own card ([`crate::panecard::pane_card`])
/// because they choose between this and the focused [`ring`].
pub(crate) fn gradient_card(
    cols: u16,
    rows: u16,
    title: &str,
    border: (u8, u8, u8),
    title_fg: (u8, u8, u8),
    bg: (u8, u8, u8),
) -> Vec<CellView> {
    let mut v = crate::boxdraw::titled_card(cols, rows, title, border, title_fg, bg);
    quiet(&mut v, cols, rows, border);
    v
}

/// Walk a card's stroke cells, handing each one its diagonal position `d`
/// (0 at the top-left corner, 1 at the bottom-right) and writing back the
/// colour the caller returns. The one place the "only cells still wearing
/// `base`, only frame glyphs" rule lives, so [`ring`] and [`quiet`] cannot
/// drift apart on it.
fn paint(
    v: &mut [CellView],
    cols: u16,
    rows: u16,
    base: (u8, u8, u8),
    color: impl Fn(f32) -> (u8, u8, u8),
) {
    let cmax = f32::from(cols.saturating_sub(1).max(1));
    let rmax = f32::from(rows.saturating_sub(1).max(1));
    for c in v.iter_mut() {
        if c.fg != base || !is_frame_glyph(c.c) {
            continue;
        }
        c.fg = color(0.5 * (f32::from(c.col) / cmax + f32::from(c.row) / rmax));
    }
}

/// `g` re-lit to `base`'s brightness: the gradient's hue, the stroke's
/// luminance.
///
/// Scaling all three channels by one factor keeps the ratios between them —
/// the hue and saturation survive, only the level moves. Where that alone
/// cannot reach the target the shortfall is made up toward white instead of
/// by clipping a channel: a clipped channel shifts the HUE, which is the one
/// thing this is protecting, while whitening only spends saturation. Light
/// themes live entirely in that second case — their `border_normal` is
/// brighter than any saturated pole can be scaled to — which is why the
/// gradient reads as a pastel there and as full colour on a dark page.
///
/// Rec.709 weights applied to the sRGB bytes directly. Gamma-space, and
/// deliberately so: this is not a contrast measurement (that is
/// `crew_theme::contrast_ratio`, which linearises properly), it is a "keep
/// this stroke as bright as it was" ratio, and both sides of the ratio are
/// measured the same way.
fn at_luma_of(g: (u8, u8, u8), base: (u8, u8, u8)) -> (u8, u8, u8) {
    let target = luma(base);
    let lg = luma(g);
    if lg < 1.0 {
        // A near-black pole carries no hue to lend; leave the stroke alone
        // rather than dividing by nothing.
        return base;
    }
    let peak = f32::from(g.0.max(g.1).max(g.2)).max(1.0);
    let k = (target / lg).min(255.0 / peak);
    let scaled = (f32::from(g.0) * k, f32::from(g.1) * k, f32::from(g.2) * k);
    let lit = 0.2126 * scaled.0 + 0.7152 * scaled.1 + 0.0722 * scaled.2;
    // Whiten only for the part scaling could not reach (light themes), and
    // never past white.
    let m = if lit < target && lit < 255.0 {
        ((target - lit) / (255.0 - lit)).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let ch = |x: f32| (x + (255.0 - x) * m).round().clamp(0.0, 255.0) as u8;
    (ch(scaled.0), ch(scaled.1), ch(scaled.2))
}

/// Rec.709 luma of an sRGB byte triple, gamma-space (see [`at_luma_of`]).
fn luma(c: (u8, u8, u8)) -> f32 {
    0.2126 * f32::from(c.0) + 0.7152 * f32::from(c.1) + 0.0722 * f32::from(c.2)
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

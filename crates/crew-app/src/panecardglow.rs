//! TRON/JARVIS light-trace chrome for CRT pane frames (goal 2026-08-04,
//! pillar 3). On the phosphor themes the focused frame stops being a flat
//! stroke and becomes a light construct: the four corner cells run hotter so
//! the bloom chain turns them into glowing nodes, gaining focus fires a
//! one-shot ignition sweep (the whole frame starts at the node colour and
//! decays to rest), and a streaming pane's frame breathes on a slow cycle.
//! Everything here is gated on `crew_theme::theme().crt.is_some()` — paper
//! themes keep their frame pixels untouched, and an idle CRT frame settles to
//! exactly `border_focused` so idle frames stay byte-identical.
use std::sync::atomic::{AtomicU32, Ordering};

use crew_render::CellView;

use crate::app::CrewApp;
use crate::pane::Pane;
use crate::panecard::{is_frame_glyph, pane_card, Bar};

/// How long the ignition sweep takes to decay from corner-hot to the resting
/// `border_focused` after a pane gains focus.
const IGNITE_MS: u64 = 600;

/// Period of the busy frame's breathing cycle. It rides the ~15fps redraws a
/// streaming pane already schedules, so it costs no frames of its own.
const BREATH_MS: u64 = 2_400;

/// How far the corner nodes lift from `border_focused` toward the hot pole.
const CORNER_LIFT: f32 = 0.5;

/// Peak of the breathing modulation — a whisper next to the corner lift, so
/// the nodes stay the brightest points on the frame even mid-breath.
const BREATH_AMP: f32 = 0.08;

/// The pole hot frame cells lean toward: over-driven white, not the theme
/// ink. On the hot phosphors (green, amber) `border_focused` already *is* the
/// ink value, so "toward ink" would move nothing; a tube driven hard whites
/// out, which is also what makes the nodes read in plain luminance.
const HOT_POLE: (u8, u8, u8) = (255, 255, 255);

/// Eased ignition progress for the frame being built, published once per
/// frame by [`CrewApp::focus_fx`] and read at the point of use — the same
/// shape as `motion::level` and `palette::accent`, so the scene-building call
/// chain doesn't grow another threaded parameter. Starts settled (1.0): an
/// app that has never ignited draws the resting frame.
static IGNITE_T: AtomicU32 = AtomicU32::new(SETTLED);
const SETTLED: u32 = 1.0f32.to_bits();

fn set_ignite_t(t: f32) {
    IGNITE_T.store(t.to_bits(), Ordering::Relaxed);
}

fn ignite_t() -> f32 {
    f32::from_bits(IGNITE_T.load(Ordering::Relaxed))
}

impl CrewApp {
    /// `build_frame`'s per-frame focus bookkeeping: diff the drawn focus
    /// (here, once per frame, so every `self.focused = …` site is caught
    /// without each having to remember to stamp a timeline), start the
    /// bracket travel and — on CRT themes only — the ignition sweep, publish
    /// the sweep's progress for [`pane_card_glowing`], and return the frame's
    /// eased bracket `focus_t`.
    pub(crate) fn focus_fx(&mut self, now: u64) -> f32 {
        if self.focus_drawn != self.focused {
            // The pane the spotlight is leaving — the dim-down side of the
            // content crossfade (see `spotlight`).
            self.focus_prev = self.focus_drawn;
            self.focus_drawn = self.focused;
            self.focus_anim = crate::ease::Timeline::start(now, 260, self.config.motion_level());
            // Ignition is CRT-only: a paper frame's pixels never change with
            // it, so spending 600ms of redraws there would buy nothing.
            if crew_theme::theme().crt.is_some() {
                self.ignite_anim =
                    crate::ease::Timeline::start(now, IGNITE_MS, self.config.motion_level());
            }
        }
        set_ignite_t(self.ignite_anim.eased(now, crate::ease::out_cubic));
        self.focus_anim.eased(now, crate::ease::out_cubic)
    }
}

/// [`pane_card`] plus the light-trace treatment: on a CRT theme the FOCUSED
/// card's frame stroke gets corner nodes, the ignition decay and — while the
/// pane is streaming — the breathing; on a MODERN theme it gets the gradient
/// light-ring instead ([`crate::modernring`]). Unfocused cards and every
/// paper theme pass through untouched (hierarchy in light: no nodes on a
/// quiet trace).
pub(crate) fn pane_card_glowing(p: &Pane, b: &Bar) -> Vec<CellView> {
    let mut v = pane_card(p.grid.cols, p.grid.rows, b);
    let theme = crew_theme::theme();
    if b.focused && theme.crt.is_some() {
        let busy = crate::paneview::pane_animating(p);
        if theme.modern.is_some() {
            crate::modernring::ring(
                &mut v,
                p.grid.cols + 2,
                p.grid.rows + 2,
                busy,
                ignite_t(),
                crate::anim::now_ms(),
            );
        } else {
            trace(
                &mut v,
                p.grid.cols + 2,
                p.grid.rows + 2,
                busy,
                ignite_t(),
                crate::anim::now_ms(),
            );
        }
    }
    v
}

/// Recolour the focused frame's stroke cells. Only cells still wearing the
/// plain `border_focused` colour are touched — the legend, focus brackets and
/// status glyphs ride the same border rows and keep their own colours.
fn trace(v: &mut [CellView], cols: u16, rows: u16, busy: bool, ignite_t: f32, now: u64) {
    let base = crew_theme::theme().border_focused;
    let hot = corner_hot(base);
    // Breathing rides the busy redraw cadence and follows the same Motion
    // gate as the scan sweep; an idle pane's breath is exactly zero, so the
    // frame returns to `border_focused` and idle frames stay byte-identical.
    let breath = if busy && crate::motion::level() != crate::motion::MotionLevel::Off {
        BREATH_AMP * crate::anim::tri(now, BREATH_MS)
    } else {
        0.0
    };
    let edge = edge_color(base, hot, ignite_t, breath);
    for c in v.iter_mut() {
        if c.fg != base || !is_frame_glyph(c.c) {
            continue;
        }
        let corner = (c.col == 0 || c.col + 1 == cols) && (c.row == 0 || c.row + 1 == rows);
        c.fg = if corner { hot } else { edge };
    }
}

/// The corner-node colour: `border_focused` lifted halfway to the hot pole,
/// so the bloom chain (bright-pass at 0.35) turns each corner into a node.
fn corner_hot(base: (u8, u8, u8)) -> (u8, u8, u8) {
    crate::anim::lerp_rgb(base, HOT_POLE, CORNER_LIFT)
}

/// The non-corner stroke colour: ignition decays it from `hot` to `base`,
/// then breathing lifts it toward the hot pole by up to [`BREATH_AMP`]. Pure
/// so it is testable, and exact at rest: `ignite_t = 1.0` with zero breath
/// returns `base` bit-for-bit (`lerp_rgb` returns its endpoint at t = 1).
fn edge_color(base: (u8, u8, u8), hot: (u8, u8, u8), ignite_t: f32, breath: f32) -> (u8, u8, u8) {
    let lit = crate::anim::lerp_rgb(hot, base, ignite_t);
    if breath <= 0.0 {
        return lit;
    }
    crate::anim::lerp_rgb(lit, HOT_POLE, breath)
}

#[cfg(test)]
#[path = "panecardglow_tests.rs"]
mod tests;

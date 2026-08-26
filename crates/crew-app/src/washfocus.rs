//! Where the page's light gathers: the wash's orbit follows the focused card.
//!
//! The background wash is two broad pools of pole light on an orbit under
//! everything, and that orbit was centred on the page — the same light no
//! matter which pane you were working in. This moves its CENTRE toward the
//! focused card, so the page is brightest under the thing you are typing
//! into and falls away from the ones you are not.
//!
//! It is wayfinding, not decoration. On a four-pane grid the focused frame is
//! one stroke among four; the wash under it is the whole lower half of the
//! window, which is why this reads from the corner of the eye when a border
//! colour does not.
//!
//! ## Why it glides, and why it glides like this
//!
//! Focus moves in one frame; a page-wide field of light that moved in one
//! frame would read as a cut, not as light. The step is the same exponential
//! smoothing [`crate::glide`] uses for pane rects, and deliberately so — a
//! pane sliding to its new tile and the light following it are one motion,
//! and two different curves would visibly disagree.
//!
//! Bounded, like everything else that moves: the smoothing converges and the
//! snap makes settling exact, so `wants_animation_frame` goes quiet again and
//! an idle crew still repaints nothing. At Motion off the step IS the snap.
//!
//! ## Determinism
//!
//! A settled focus is a constant, so a still frame is still a pure function
//! of pixel position — the contract the CRT trace, the gradient ring and every
//! headless shot test keep. A crew that has never focused anything (and every
//! test that builds an app and renders once) sits at [`PULL_NONE`], which is
//! the page centre: exactly the pre-v0.18.34 wash.
use crate::layout::Rect;
use crate::motion::MotionLevel;

/// How far the orbit travels toward the focused card, as a fraction of the
/// distance from the page centre to that card's centre.
///
/// Not 1.0: the pools are wider than a pane, so parking them dead on the
/// card's centre puts the falloff *inside* the card and the light stops
/// reading as a page-wide field. At 0.55 the gather is unmistakable on a
/// 2×2 grid and still looks like one continuous wash.
pub(crate) const PULL: f32 = 0.55;

/// The pull of a frame with nothing focused — no move at all, the page
/// centre, the wash crew had before this existed.
pub(crate) const PULL_NONE: f32 = 0.0;

/// Smoothing time constant, ms: after this long the light has covered ~63% of
/// the remaining distance. Slower than [`crate::glide`]'s 90 ms on purpose —
/// a pane arrives, and light *fills in* behind it.
const TAU_MS: f32 = 160.0;

/// Distance (in uv, so a fraction of the window) under which the centre snaps
/// exactly. A twentieth of a percent of the page: far under one pixel of
/// visible movement in a field this soft, so the snap can never be seen.
const SNAP_UV: f32 = 0.0005;

/// Where the wash's orbit is centred right now, in uv, plus how far along it
/// is toward the focused card. `Default` is the settled, unfocused state —
/// the page centre — which is what an app that has never drawn should hold.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WashFocus {
    /// Orbit centre in uv (`0.5, 0.5` = the page centre).
    at: (f32, f32),
    /// Live pull, glided toward `PULL` while something is focused and toward
    /// `PULL_NONE` when nothing is.
    pull: f32,
    /// Whether the last step left anything still moving. Read by
    /// `wants_animation_frame`, so a live glide keeps asking for frames and a
    /// settled one asks for none.
    moving: bool,
}

impl Default for WashFocus {
    fn default() -> Self {
        Self {
            at: (0.5, 0.5),
            pull: PULL_NONE,
            moving: false,
        }
    }
}

impl WashFocus {
    /// The uniform the background pass wants: `(centre_uv, pull)`.
    pub(crate) fn uniform(&self) -> ((f32, f32), f32) {
        (self.at, self.pull)
    }

    /// Whether this glide still has frames to draw.
    pub(crate) fn moving(&self) -> bool {
        self.moving
    }

    /// One frame of follow. `focused` is the focused card's rect in the same
    /// pixel space as `surface`; `None` (nothing focused, or a surface with no
    /// area) glides the pull back to nothing rather than snapping the light
    /// home, so closing the last pane dims the gather instead of cutting it.
    pub(crate) fn step(
        &mut self,
        focused: Option<Rect>,
        surface: (f32, f32),
        dt_ms: u64,
        motion: MotionLevel,
    ) {
        let (target_at, target_pull) = match target_uv(focused, surface) {
            // Hold the last centre while fading out: a pull heading for zero
            // makes the centre irrelevant, and moving both at once would swing
            // the light across the page on its way to standing still.
            None => (self.at, PULL_NONE),
            Some(at) => (at, PULL),
        };
        if motion == MotionLevel::Off {
            *self = Self {
                at: target_at,
                pull: target_pull,
                moving: false,
            };
            return;
        }
        let k = 1.0 - (-(dt_ms as f32) / TAU_MS).exp();
        let m = |a: f32, b: f32| a + (b - a) * k;
        let at = (m(self.at.0, target_at.0), m(self.at.1, target_at.1));
        let pull = m(self.pull, target_pull);
        let settled = (at.0 - target_at.0).abs() < SNAP_UV
            && (at.1 - target_at.1).abs() < SNAP_UV
            && (pull - target_pull).abs() < SNAP_UV;
        *self = if settled {
            Self {
                at: target_at,
                pull: target_pull,
                moving: false,
            }
        } else {
            Self {
                at,
                pull,
                moving: true,
            }
        };
    }
}

/// The focused card's centre in uv, or `None` when there is nothing to follow
/// — no focused rect, an empty surface, or a degenerate rect. Clamped to the
/// page: a card hanging off the edge mid-glide must not throw the light off
/// with it.
fn target_uv(focused: Option<Rect>, (sw, sh): (f32, f32)) -> Option<(f32, f32)> {
    let r = focused?;
    if sw <= 0.0 || sh <= 0.0 || r.w <= 0.0 || r.h <= 0.0 {
        return None;
    }
    Some((
        ((r.x + r.w / 2.0) / sw).clamp(0.0, 1.0),
        ((r.y + r.h / 2.0) / sh).clamp(0.0, 1.0),
    ))
}

#[cfg(test)]
#[path = "washfocus_tests.rs"]
mod tests;

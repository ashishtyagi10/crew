//! Grid reflow animation: panes *glide* to their new tiles instead of
//! snapping. Opening, closing, minimizing or restoring a pane reflows the
//! whole grid — until now every survivor teleported to its new rect in one
//! frame, which reads as a different layout, not the same panes rearranging.
//!
//! Each pane's four edges are [`crate::spring`]s. That is the whole design:
//! `pane.rect` is still the rect the pane was drawn at last frame, but the
//! velocity that got it there is carried alongside it, so a grid that changes
//! again mid-reflow curves through rather than kinking. Exponential smoothing,
//! which this used to be, cannot do that — it integrates from position alone,
//! so a retarget is indistinguishable from a fresh start at rest, and closing
//! two panes in quick succession looked like two unrelated animations.
//!
//! Still deliberately identity-free: a pane closed mid-glide simply stops
//! being stepped, and a restored pane glides out of wherever its rect last was
//! (its old tile, or the spot it held before minimizing). Zoom keeps its own
//! lerp; unzoom lands in this path and springs back to the tile for free.
//!
//! Bounded: a critically damped spring converges and the snap makes settling
//! exact, so `wants_animation_frame` (via `CrewApp::glide_active`) goes quiet
//! — the "an idle crew never repaints" invariant holds. At Motion off the step
//! is the snap itself.
use crate::layout::Rect;
use crate::spring::Spring;

/// Stiffness of the grid's springs, in rad/s.
///
/// Tuned to land in roughly the quarter-second the old smoothing took (it had
/// a 90ms time constant), so the reflow keeps the pace people are used to and
/// only its *character* changes — weight on the way out, a settle on the way
/// in, and continuity when the grid changes again mid-flight.
const OMEGA: f32 = 18.0;

/// A rect whose four edges are springs — the state a glide integrates from.
///
/// Four independent springs, not one: a pane moving right while growing taller
/// is two different distances at two different speeds, and a single scalar
/// would have to pick one of them to be wrong about.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Glide {
    x: Spring,
    y: Spring,
    w: Spring,
    h: Spring,
}

impl Glide {
    /// A glide resting exactly on `r`.
    pub(crate) fn at(r: Rect) -> Self {
        Self {
            x: Spring::at(r.x),
            y: Spring::at(r.y),
            w: Spring::at(r.w),
            h: Spring::at(r.h),
        }
    }

    /// The rect this glide currently holds.
    pub(crate) fn rect(&self) -> Rect {
        Rect {
            x: self.x.pos,
            y: self.y.pos,
            w: self.w.pos,
            h: self.h.pos,
        }
    }

    /// One frame toward `target` over `dt_ms`. Returns the rect to draw and
    /// whether every edge has arrived and stopped.
    ///
    /// Snaps outright when motion is off, when the pane has no prior rect
    /// (a fresh spawn — the assemble animation owns entrances), or when the
    /// glide has been reseeded from a teleported rect.
    pub(crate) fn step(&mut self, target: Rect, dt_ms: u64, snap: bool) -> (Rect, bool) {
        let cur = self.rect();
        if snap || cur.w <= 0.0 || cur.h <= 0.0 {
            *self = Glide::at(target);
            return (target, true);
        }
        let dt = dt_ms as f32;
        self.x.step(target.x, dt, OMEGA);
        self.y.step(target.y, dt, OMEGA);
        self.w.step(target.w, dt, OMEGA);
        self.h.step(target.h, dt, OMEGA);
        let settled = self.x.settled(target.x)
            && self.y.settled(target.y)
            && self.w.settled(target.w)
            && self.h.settled(target.h);
        if settled {
            // Park exactly, so a resting layout is the placed layout and not
            // a fraction of a pixel off it.
            self.x.snap_to(target.x);
            self.y.snap_to(target.y);
            self.w.snap_to(target.w);
            self.h.snap_to(target.h);
            return (target, true);
        }
        (self.rect(), false)
    }

    /// Adopt `r` as the current position with no velocity.
    ///
    /// The seam between this and everything else that writes `pane.rect`:
    /// zoom's own lerp, a drag, a resize. Those move a pane without the
    /// spring's knowledge, and a spring that kept integrating from a stale
    /// position would yank the pane back toward where it thought it was.
    pub(crate) fn reseed(&mut self, r: Rect) {
        *self = Glide::at(r);
    }
}

/// Clamp a frame delta to something sane: the first frame after an idle
/// stretch would otherwise be integrated as one enormous step (and while the
/// spring substeps rather than exploding, it would still cover the whole
/// distance at once — exactly the teleport this module exists to remove).
pub(crate) fn frame_dt(now: u64, prev: u64) -> u64 {
    now.saturating_sub(prev).min(100)
}

#[cfg(test)]
#[path = "glide_tests.rs"]
mod tests;

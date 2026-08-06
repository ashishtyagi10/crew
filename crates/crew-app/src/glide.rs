//! Grid reflow animation: panes *glide* to their new tiles instead of
//! snapping. Opening, closing, minimizing or restoring a pane reflows the
//! whole grid — until now every survivor teleported to its new rect in one
//! frame, which reads as a different layout, not the same panes rearranging.
//!
//! The mechanism is deliberately stateless: `pane.rect` already holds the rect
//! the pane was *drawn at* last frame, so each frame moves it a time-scaled
//! fraction of the way toward the placed target (exponential smoothing) and
//! snaps once within a pixel. No per-pane timelines, no identity bookkeeping —
//! a pane closed mid-glide simply stops being stepped, and a restored pane
//! glides out of wherever its rect last was (its old tile, or the spot it held
//! before minimizing). Zoom keeps its own lerp; unzoom lands in this path and
//! glides back to the tile for free.
//!
//! Bounded: smoothing converges and the snap makes settling exact, so
//! `wants_animation_frame` (via `CrewApp::glide_active`) goes quiet — the
//! "an idle crew never repaints" invariant holds. At Motion off the step is
//! the snap itself.
use crate::layout::Rect;

/// Smoothing time constant: after `TAU_MS` the pane has covered ~63% of the
/// remaining distance; visually settled in ~250ms.
const TAU_MS: f32 = 90.0;

/// Distance (px) under which every edge snaps to the target exactly.
const SNAP_PX: f32 = 0.8;

/// One frame of glide: move `current` toward `target` for a frame `dt_ms`
/// long. Returns the rect to draw this frame and whether it has settled.
/// Snaps outright when motion is off, when the pane has no prior rect (fresh
/// spawn — the assemble animation owns entrances), or on a stale `dt` (first
/// frame after idle).
pub(crate) fn step(current: Rect, target: Rect, dt_ms: u64, snap: bool) -> (Rect, bool) {
    if snap || current.w <= 0.0 || current.h <= 0.0 {
        return (target, true);
    }
    let k = 1.0 - (-(dt_ms as f32) / TAU_MS).exp();
    let m = |a: f32, b: f32| a + (b - a) * k;
    let stepped = Rect {
        x: m(current.x, target.x),
        y: m(current.y, target.y),
        w: m(current.w, target.w),
        h: m(current.h, target.h),
    };
    let close = |a: f32, b: f32| (a - b).abs() < SNAP_PX;
    let settled = close(stepped.x, target.x)
        && close(stepped.y, target.y)
        && close(stepped.w, target.w)
        && close(stepped.h, target.h);
    if settled {
        (target, true)
    } else {
        (stepped, false)
    }
}

/// Clamp a frame delta to something sane: the first frame after an idle
/// stretch would otherwise cover the whole distance at once (k → 1), which is
/// exactly the teleport this module exists to remove.
pub(crate) fn frame_dt(now: u64, prev: u64) -> u64 {
    now.saturating_sub(prev).min(100)
}

#[cfg(test)]
#[path = "glide_tests.rs"]
mod tests;

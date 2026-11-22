//! How a toast leaves, and which frames that costs.
//!
//! With motion on, the last [`EXIT_MS`] of a card's life slide it back out
//! past the right edge while its text dissolves, and every frame of that
//! window is drawn. At Motion off there is nothing to draw in that window:
//! the card holds still, whole, and the one frame it is owed is the one that
//! takes it away once its time is up — asked for by the poll's prune. Before this, Motion off still dissolved
//! the text — asking for ~20 frames over 340 ms — and then stopped asking
//! before the frame that removed it, so an empty card box stayed on screen
//! until something else happened to repaint.
use crate::motion::MotionLevel;
use crate::toast::{EXIT_MS, TTL_MS};

/// The card's sideways offset (px, toward the right edge) and how far its
/// text has dissolved (0 whole … 1 gone), `age` ms into its life.
pub(crate) fn leave(age: u64, travel: f32, level: MotionLevel) -> (f32, f32) {
    if level == MotionLevel::Off {
        return (0.0, 0.0);
    }
    let e = (age.saturating_sub(TTL_MS - EXIT_MS) as f32 / EXIT_MS as f32).clamp(0.0, 1.0);
    (e.powi(2) * travel, e)
}

/// Whether a card `age` ms old has a frame to draw now: sliding in or out
/// with motion on; at Motion off never — the frame that removes it is owed
/// by the poll's own prune (`Toasts::expire`), which is bounded by the
/// clock, where "past its time" stays true until something draws.
pub(crate) fn wants_frame(age: u64, sliding: bool, level: MotionLevel) -> bool {
    level != MotionLevel::Off && age < TTL_MS && (sliding || age >= TTL_MS - EXIT_MS)
}

#[cfg(test)]
#[path = "toastexit_tests.rs"]
mod tests;

//! The glass rims' light leans toward the pointer.
//!
//! Every sheet is lit from the upper left (crew-render's `glass.wgsl`), and a
//! light that never moves is a painted highlight. Liquid glass on a phone
//! answers the hand that tilts it; on a desktop the hand is the pointer. So
//! the rim light TILTS with it — a pointer at the right edge brings the light
//! round toward the right, one near the bottom lowers it — never flipping
//! the light entirely: the shader only leans its fixed upper-left light
//! toward this tilt, so the look stays the look.
//!
//! Bounded like [`crate::washfocus`]: the tilt glides with the same kind of
//! exponential smoothing and snaps when it arrives, so `wants_animation_frame`
//! goes quiet again and a still pointer costs no frames. A pointer that
//! leaves the window lets the light drift home to its resting direction.
use crate::motion::MotionLevel;

/// Smoothing time constant, ms. Slow on purpose: light that snapped to the
/// pointer would read as a cursor effect, not as light.
const TAU_MS: f32 = 220.0;

/// Tilt distance under which the glide snaps exactly — far under a visible
/// change in a rim a few pixels wide.
const SNAP: f32 = 0.002;

/// How finely a pointer move is noticed, as a fraction of the window. A move
/// inside the same step asks for no frame, so a pointer resting on a card and
/// jittering a pixel repaints nothing.
const STEP: f32 = 1.0 / 32.0;

/// The rim light's tilt, `(-1..=1, -1..=1)` across the window, `(0, 0)` at
/// rest (the pointer outside, or never seen).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct PointerLight {
    at: (f32, f32),
    target: (f32, f32),
    moving: bool,
}

impl PointerLight {
    /// The tilt the glass pass is handed this frame.
    pub(crate) fn tilt(&self) -> (f32, f32) {
        self.at
    }

    /// Whether the glide still has frames to draw.
    pub(crate) fn moving(&self) -> bool {
        self.moving
    }

    /// Aim at the pointer at `cursor` in a `surface`-sized window, or home
    /// when it is `None`. Returns whether the aim moved by a whole [`STEP`] —
    /// the caller's cue to ask for a frame.
    pub(crate) fn aim(&mut self, cursor: Option<(f32, f32)>, surface: (f32, f32)) -> bool {
        let target = match cursor {
            Some((x, y)) if surface.0 > 0.0 && surface.1 > 0.0 => (
                (x / surface.0 * 2.0 - 1.0).clamp(-1.0, 1.0),
                (y / surface.1 * 2.0 - 1.0).clamp(-1.0, 1.0),
            ),
            _ => (0.0, 0.0),
        };
        let q = |v: f32| (v / (2.0 * STEP)).round();
        let moved = q(target.0) != q(self.target.0) || q(target.1) != q(self.target.1);
        if moved {
            self.target = target;
            self.moving = true;
        }
        moved
    }

    /// One frame of glide toward the aim.
    pub(crate) fn step(&mut self, dt_ms: u64, motion: MotionLevel) {
        if !self.moving {
            return;
        }
        let k = match motion {
            MotionLevel::Off => 1.0,
            _ => 1.0 - (-(dt_ms as f32) / TAU_MS).exp(),
        };
        let m = |a: f32, b: f32| a + (b - a) * k;
        let at = (m(self.at.0, self.target.0), m(self.at.1, self.target.1));
        let settled = (at.0 - self.target.0).abs() < SNAP && (at.1 - self.target.1).abs() < SNAP;
        self.at = if settled { self.target } else { at };
        self.moving = !settled;
    }
}

impl crate::app::CrewApp {
    /// Whether anything that follows the pointer — the rims' light, the
    /// hovered card's lift — is still gliding (`wants_animation_frame`).
    pub(crate) fn pointer_gliding(&self) -> bool {
        self.pointer_light.moving() || self.hover_lift.moving()
    }
}

#[cfg(test)]
#[path = "pointerlight_tests.rs"]
mod tests;

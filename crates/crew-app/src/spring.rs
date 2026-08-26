//! A spring, not a fade — the motion primitive with a memory.
//!
//! [`crate::ease`] and the grid's exponential smoothing both animate by
//! *interpolating*: given where a thing is and where it should be, they cover
//! some fraction of the difference. That is fine for anything that starts at
//! rest and arrives once, and wrong for anything that can be redirected while
//! it is still moving — which is the whole life of a pane grid. Close a pane
//! while the last close is still reflowing and a smoothed rect kinks: it
//! forgets it was travelling and starts a fresh decay from wherever it
//! happened to be.
//!
//! A spring carries **velocity**. Retarget it mid-flight and the motion
//! curves through, because the state it integrates from is (position,
//! velocity) rather than position alone. This is why every modern motion
//! system — SwiftUI's `.spring`, Framer Motion, the Web Animations spring
//! proposals — settled on the same primitive: it is not a nicer curve, it is
//! the only curve that survives interruption.
//!
//! Crew's springs are **critically damped** (ζ = 1): the fastest approach
//! that does not overshoot. A pane that bounces past its tile and comes back
//! would be reading as playful about a layout change the user asked for, and
//! this canvas is not that. What the spring buys here is the interruption
//! behaviour and the weight of the arrival, not a bounce.
//!
//! Bounded, like every other animation in crew: [`Spring::settled`] is
//! position AND velocity, so a spring that has arrived but is still moving is
//! not finished, and one that has stopped is — after which nothing reschedules
//! a frame.

/// Integration substep. Semi-implicit Euler is stable while `dt · ω` stays
/// small; a 100ms frame against a 90ms half-life is not small, so long frames
/// are integrated in pieces rather than trusted whole. The alternative — an
/// analytic solution — is exact but has to be re-derived for every damping
/// ratio, and this is a display, not a physics engine.
const MAX_STEP_MS: f32 = 8.0;

/// Position within which a spring may be considered arrived, in px.
const SNAP_PX: f32 = 0.5;

/// Speed under which a spring may be considered stopped, in px per second.
///
/// Both bounds are needed. Position alone calls a spring passing *through*
/// its target at speed "settled", which snaps it dead mid-flight — the exact
/// teleport the animation exists to remove.
const SNAP_VEL: f32 = 12.0;

/// One axis of springy motion: where it is, and how fast it is going.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Spring {
    pub(crate) pos: f32,
    pub(crate) vel: f32,
}

impl Spring {
    /// A spring at rest at `pos`.
    pub(crate) fn at(pos: f32) -> Self {
        Self { pos, vel: 0.0 }
    }

    /// Integrate toward `target` for `dt_ms`, with angular frequency
    /// `omega` (rad/s). Higher `omega` is a stiffer, faster spring.
    pub(crate) fn step(&mut self, target: f32, dt_ms: f32, omega: f32) {
        let mut left = dt_ms.max(0.0);
        while left > 0.0 {
            let h = left.min(MAX_STEP_MS) / 1000.0;
            left -= MAX_STEP_MS;
            // Critical damping: a = -ω²·x - 2ω·v. Velocity is updated first
            // and position from the NEW velocity (semi-implicit Euler), which
            // is what keeps the integration from feeding itself energy.
            let x = self.pos - target;
            let a = -omega * omega * x - 2.0 * omega * self.vel;
            self.vel += a * h;
            self.pos += self.vel * h;
        }
    }

    /// Whether this spring has arrived AND stopped.
    pub(crate) fn settled(&self, target: f32) -> bool {
        (self.pos - target).abs() < SNAP_PX && self.vel.abs() < SNAP_VEL
    }

    /// Park exactly on `target`, killing the velocity. Called once a spring
    /// settles so the resting value is exact rather than nearly.
    pub(crate) fn snap_to(&mut self, target: f32) {
        self.pos = target;
        self.vel = 0.0;
    }
}

#[cfg(test)]
#[path = "spring_tests.rs"]
mod tests;

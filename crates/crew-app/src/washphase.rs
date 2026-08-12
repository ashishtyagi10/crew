//! The clock behind the modern backdrop's gradient wash: where the two pools
//! of pole light sit on their orbit this frame.
//!
//! The wash itself is drawn by the background pass (see crew-render's
//! `ModernPaper`); all that lives here is the one number it rotates by, and
//! the rule for when that number is allowed to move. Crew does not repaint an
//! idle window — animation frames only exist while a pane is busy — so the
//! phase is advanced from ELAPSED TIME BETWEEN BUSY FRAMES rather than read
//! off the wall clock. Two consequences, both deliberate:
//!
//! * The aurora drifts only while something is actually happening, and holds
//!   wherever it stopped once things go quiet. An idle frame stays a pure
//!   function of pixel position — the same static-frame determinism contract
//!   the CRT trace and the modern ring keep.
//! * It never jumps. A wall-clock phase would teleport the pools across the
//!   page after a quiet minute; accumulating deltas means the motion the user
//!   sees is continuous across every pause.
use crate::motion::MotionLevel;

/// Longest delta a single frame may contribute, in ms. A frame gap longer
/// than this is a stall (a slow build, a laptop lid, a blocking read on the
/// winit thread), and paying it back in one step would show up as a lurch.
const MAX_STEP_MS: u64 = 250;

/// The wash's orbital position, in turns.
#[derive(Default)]
pub(crate) struct WashPhase {
    phase: f32,
    /// When the last BUSY frame was stamped. Cleared whenever the wash holds,
    /// so the first frame of the next burst of activity contributes nothing
    /// and the quiet time in between is never paid back.
    last_ms: Option<u64>,
}

impl WashPhase {
    /// This frame's phase. Advances by the time since the previous busy frame
    /// when `busy` (one full revolution per `drift_ms`), holds otherwise —
    /// including at Motion off, which is a genuine off, not a slow setting.
    /// The motion level is passed in rather than read from the global so the
    /// clock is a pure function of its inputs.
    pub(crate) fn advance(
        &mut self,
        now_ms: u64,
        busy: bool,
        drift_ms: u64,
        motion: MotionLevel,
    ) -> f32 {
        if !busy || drift_ms == 0 || motion == MotionLevel::Off {
            self.last_ms = None;
            return self.phase;
        }
        let dt = self
            .last_ms
            .map_or(0, |last| now_ms.saturating_sub(last).min(MAX_STEP_MS));
        self.last_ms = Some(now_ms);
        self.phase = (self.phase + dt as f32 / drift_ms as f32).fract();
        self.phase
    }
}

#[cfg(test)]
#[path = "washphase_tests.rs"]
mod tests;

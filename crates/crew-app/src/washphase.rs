//! The clock behind the modern backdrop's gradient wash: where the two pools
//! of pole light sit on their orbit this frame.
//!
//! The wash itself is drawn by the background pass (see crew-render's
//! `ModernPaper`); all that lives here is the one number it rotates by, and
//! the rule for when that number is allowed to move — its **pace**, in ms per
//! revolution, or `None` to hold.
//!
//! The phase is advanced from ELAPSED TIME BETWEEN DRAWN FRAMES rather than
//! read off the wall clock, so it never jumps: a wall-clock phase would
//! teleport the pools across the page after a quiet minute, while accumulating
//! deltas means the motion is continuous across every pause.
//!
//! ## Two paces
//!
//! **Busy** is the original one: a revolution per the theme's `drift_ms` (six
//! seconds), riding frames that activity was already drawing, so it cost
//! nothing.
//!
//! **Ambient** is the one that makes a quiet window feel alive rather than
//! frozen. It is [`AMBIENT_MULT`] times slower, and it is the only motion in
//! crew that asks for frames nothing else needed — which is why it is fenced:
//! the user's `ambient_drift` setting, Motion not off, a theme that has a wash
//! at all, and crew holding the OS focus. A window you are not looking at
//! repaints for nobody.
//!
//! Turning ambient off restores the old behaviour exactly, including the
//! static-frame determinism the CRT trace and the modern ring keep — which is
//! also what every headless shot test relies on.
use crate::motion::MotionLevel;

/// Longest delta a single frame may contribute, in ms. A frame gap longer
/// than this is a stall (a slow build, a laptop lid, a blocking read on the
/// winit thread), and paying it back in one step would show up as a lurch.
const MAX_STEP_MS: u64 = 250;

/// How much slower the idle drift is than the busy one. At the themes'
/// 6 s `drift_ms` this is a revolution every 90 seconds — about 4° of orbit
/// per second, which reads as a room slowly changing light rather than
/// anything that wants your attention. Busy motion is a signal and should be
/// noticed; ambient motion is a texture and should not be.
pub(crate) const AMBIENT_MULT: u64 = 15;

/// Ms per revolution this frame, or `None` to hold where it is.
///
/// `busy` wins over `ambient`: a working pane's wash keeps its own faster
/// pace, so the two never fight over one phase, and stepping between them is
/// continuous because both accumulate onto the same number.
pub(crate) fn pace(drift_ms: u64, busy: bool, ambient: bool) -> Option<u64> {
    if drift_ms == 0 {
        return None;
    }
    match (busy, ambient) {
        (true, _) => Some(drift_ms),
        (false, true) => Some(drift_ms.saturating_mul(AMBIENT_MULT)),
        (false, false) => None,
    }
}

/// The wash's orbital position, in turns.
#[derive(Default)]
pub(crate) struct WashPhase {
    phase: f32,
    /// When the last DRIFTING frame was stamped. Cleared whenever the wash
    /// holds, so the first frame after a hold contributes nothing and the
    /// still time in between is never paid back.
    last_ms: Option<u64>,
}

impl WashPhase {
    /// This frame's phase. Advances by the time since the previous drawn
    /// frame at `pace` ms per revolution ([`pace`] decides which), holds on
    /// `None` — and holds at Motion off, which is a genuine off and not a slow
    /// setting. The motion level is passed in rather than read from the global
    /// so the clock is a pure function of its inputs.
    pub(crate) fn advance(&mut self, now_ms: u64, pace: Option<u64>, motion: MotionLevel) -> f32 {
        let Some(pace) = pace.filter(|&p| p > 0 && motion != MotionLevel::Off) else {
            self.last_ms = None;
            return self.phase;
        };
        let dt = self
            .last_ms
            .map_or(0, |last| now_ms.saturating_sub(last).min(MAX_STEP_MS));
        self.last_ms = Some(now_ms);
        self.phase = (self.phase + dt as f32 / pace as f32).fract();
        self.phase
    }
}

impl crate::app::CrewApp {
    /// Whether the page's wash should drift on its own this frame.
    ///
    /// Four fences, all of which must pass. The setting, because ambient
    /// motion is a taste and some people want a still window. Motion not off,
    /// which is a genuine off. A theme that actually has a wash to move —
    /// moving a phase nothing reads would buy frames for no pixels. And the
    /// OS focus, because the whole cost of this feature is repainting a window
    /// that would otherwise be asleep, and it is only worth paying while
    /// someone is looking at it.
    pub(crate) fn ambient_drift(&self) -> bool {
        self.config.ambient_drift
            && self.win_focus.unwrap_or(true)
            && crate::motion::level() != MotionLevel::Off
            && crew_theme::theme().modern.is_some_and(|m| m.wash > 0.0)
    }
}

#[cfg(test)]
#[path = "washphase_tests.rs"]
mod tests;

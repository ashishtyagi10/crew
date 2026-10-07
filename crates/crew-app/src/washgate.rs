//! When the page's wash may move on its own, and how often it is drawn while
//! it does — the fences around [`crate::washphase`]'s clocks.
use crate::motion::MotionLevel;
use crate::washphase::AMBIENT_MULT;

/// How much slower than the theme's `drift_ms` the page moves while a pane
/// works: twice, a revolution every twelve seconds — so busy is still twice
/// the idle pace, a signal you can see, without the backdrop racing behind
/// the very output you are reading. (It was once, four times the idle pace.)
pub(crate) const BUSY_MULT: u64 = 2;

/// Ms per revolution this frame, or `None` to hold where it is.
///
/// `busy` wins over `ambient`: a working pane's wash keeps its own faster
/// pace, so the two never fight over one phase, and stepping between them is
/// continuous because both accumulate onto the same number — and a glide,
/// because the clocks reach a new pace through washphase's flywheel.
pub(crate) fn pace(drift_ms: u64, busy: bool, ambient: bool) -> Option<u64> {
    if drift_ms == 0 {
        return None;
    }
    match (busy, ambient) {
        (true, _) => Some(drift_ms.saturating_mul(BUSY_MULT)),
        (false, true) => Some(drift_ms.saturating_mul(AMBIENT_MULT)),
        (false, false) => None,
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

    /// Poll ticks per frame while something is in flight: the backdrop's own
    /// smooth rate whenever the page drifts, so its faster busy sway is never
    /// drawn choppier than its idle one; the progress sweep's otherwise.
    pub(crate) fn busy_anim_div(&self) -> u64 {
        if self.ambient_drift() {
            crate::poll::AMBIENT_ANIM_DIV
        } else {
            crate::poll::BUSY_ANIM_DIV
        }
    }
}

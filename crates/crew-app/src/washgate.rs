//! When the page's wash may move on its own, and how often it is drawn while
//! it does — the fences around [`crate::washphase`]'s clocks.
use crate::motion::MotionLevel;

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

    /// Poll ticks per frame while something is in flight: the vortex's own
    /// smooth rate whenever the page drifts, so its faster busy spin is never
    /// drawn choppier than its idle one; the progress sweep's otherwise.
    pub(crate) fn busy_anim_div(&self) -> u64 {
        if self.ambient_drift() {
            crate::poll::AMBIENT_ANIM_DIV
        } else {
            crate::poll::BUSY_ANIM_DIV
        }
    }
}

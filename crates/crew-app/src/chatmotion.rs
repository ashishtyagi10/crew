//! The crew pane's chrome motion, as one bounded predicate for the busy
//! branch of `poll`'s redraw registry (`panebusy::pane_animating`).
//!
//! Each of these is short-lived by construction — a timeline that settles,
//! or a state the pane leaves on its own — so an idle pane reports nothing
//! and never repaints for chrome that is not moving:
//!
//! - the `↓ N new` pill's pop (`chatpop`, [`crate::chatpop::POP_MS`]);
//! - the queued hourglass turning (`chatqueue`) — while the queue holds
//!   anything, which it stops doing the moment the pane goes idle and
//!   `poll` flushes it;
//! - the progress bar's fill sweep and the counter's digit flash
//!   (`chatprogspring`, `chatflash`) — on the live run, which is busy anyway;
//! - a working badge's token pulse (`summarypulse`), for
//!   [`crate::shimmer::PULSE_MS`] after each burst;
//! - a pressed plan button's invert flash (`chatplanbtn::PRESS_MS`).
//!
//! Off is a genuine off: nothing here reports a frame at Off, since every
//! timeline is born settled and the hourglass stands still.
use crate::chat::ChatPane;
use crate::motion::MotionLevel;

impl ChatPane {
    /// Whether some piece of this pane's chrome is mid-animation at `now`
    /// (the animation clock).
    pub(crate) fn chrome_animating(&self, now: u64) -> bool {
        let level = crate::motion::level();
        if level == MotionLevel::Off {
            return false;
        }
        self.pill_pop.live(now)
            || self.press_btn.is_some_and(|(_, flash)| flash.live(now))
            || !self.queued.is_empty()
            || self
                .swarm
                .as_ref()
                .is_some_and(|s| crate::chatprogspring::live(s, now) || s.flash.live(now))
            || self
                .token_pulse
                .values()
                .any(|&t| now.saturating_sub(t) < level.scale_ms(crate::shimmer::PULSE_MS))
    }
}

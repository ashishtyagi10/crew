//! Confirm-to-quit guard: when panes (shells/agents) are open, the first quit
//! press only arms a short window and flashes a status; a second press within
//! it actually exits. With no panes open, quitting is immediate.
use std::time::{Duration, Instant};

use crate::app::CrewApp;

/// How long the "press quit again" confirmation stays armed.
const QUIT_WINDOW: Duration = Duration::from_secs(2);

/// Whether to exit now: immediately when nothing is open, otherwise only if a
/// previous quit press is still within the confirmation window.
fn quit_decision(has_panes: bool, armed: Option<Instant>, now: Instant) -> bool {
    if !has_panes {
        return true;
    }
    armed.is_some_and(|t| now.duration_since(t) < QUIT_WINDOW)
}

impl CrewApp {
    /// Returns `true` if the app should exit now. Otherwise arms a 2s confirm
    /// window and flashes a status so a stray keypress can't kill live sessions.
    pub(crate) fn confirm_quit(&mut self) -> bool {
        let now = Instant::now();
        if quit_decision(!self.panes.is_empty(), self.quit_armed, now) {
            return true;
        }
        self.quit_armed = Some(now);
        self.set_status("press quit again to exit");
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_panes_exits_immediately() {
        assert!(quit_decision(false, None, Instant::now()));
    }

    #[test]
    fn first_press_with_panes_does_not_exit() {
        assert!(!quit_decision(true, None, Instant::now()));
    }

    #[test]
    fn second_press_within_window_exits() {
        let now = Instant::now();
        // armed just now → still within the confirmation window
        assert!(quit_decision(true, Some(now), now));
    }
}

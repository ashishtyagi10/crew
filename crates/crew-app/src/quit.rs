//! Confirm-to-quit guard: when panes (shells/agents) are open, the first quit
//! REQUEST only arms a short window and flashes a status; a second within it
//! actually exits. With no panes open, quitting is immediate.
//!
//! Every way out of the app funnels through here — Cmd+Q/Ctrl+Q *and* the
//! window close button. The close button used to call `event_loop.exit()`
//! directly, so the guard that stopped a stray keystroke from killing running
//! shells did nothing about a stray click on the traffic light, which is the
//! easier of the two to hit by accident.
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

/// The confirmation prompt, naming what is actually at stake.
fn quit_prompt(panes: usize) -> String {
    let s = if panes == 1 { "" } else { "s" };
    format!("quit again to exit — {panes} pane{s} open")
}

impl CrewApp {
    /// Returns `true` if the app should exit now. Otherwise arms a 2s confirm
    /// window and flashes a status so a stray keypress — or a stray click on
    /// the close button — can't kill live sessions.
    pub(crate) fn confirm_quit(&mut self) -> bool {
        let now = Instant::now();
        // Cmd+Q takes the whole app, so what is at stake is every pane in
        // every window — not just this one's. `canvas` stamps the others'
        // count before routing the event, because a prompt that says "1 pane
        // open" while a second window is running three agents is worse than
        // no prompt at all.
        let open = self.panes.len() + self.other_panes;
        if quit_decision(open > 0, self.quit_armed, now) {
            return true;
        }
        self.quit_armed = Some(now);
        // Deliberately action-neutral: this same prompt answers Cmd+Q and the
        // window close button, so it can't say "press" or "click". Naming the
        // count is the part that makes it a decision rather than a nag.
        self.set_status(quit_prompt(open));
        false
    }
}

#[cfg(test)]
#[path = "quit_tests.rs"]
mod tests;

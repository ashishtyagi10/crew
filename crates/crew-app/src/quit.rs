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
        if quit_decision(!self.panes.is_empty(), self.quit_armed, now) {
            return true;
        }
        self.quit_armed = Some(now);
        // Deliberately action-neutral: this same prompt answers Cmd+Q and the
        // window close button, so it can't say "press" or "click". Naming the
        // count is the part that makes it a decision rather than a nag.
        self.set_status(quit_prompt(self.panes.len()));
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

    /// A stale arm must not still be live: walking away and clicking close an
    /// hour later has to ask again, not exit on the first click.
    #[test]
    fn an_expired_arm_does_not_exit() {
        let now = Instant::now();
        let stale = now - QUIT_WINDOW - Duration::from_millis(1);
        assert!(!quit_decision(true, Some(stale), now));
    }

    fn app_with_a_pane() -> CrewApp {
        use crate::pane::{Pane, PaneContent};
        use crew_term::GridSize;
        let mut app = CrewApp::default();
        app.panes.push(Pane {
            content: PaneContent::Far(crate::farpane::FarPane::new(std::env::temp_dir())),
            grid: GridSize { cols: 80, rows: 24 },
            rect: crate::layout::Rect {
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: 0.0,
            },
            label: Some("pane".into()),
            name: None,
            dir: None,
            activity: false,
            bell: false,
            hidden: false,
            attention: None,
            born_ms: crate::anim::now_ms(),
        });
        app
    }

    /// The guard both the close button and Cmd+Q now share: with a pane open
    /// the first request arms and explains, the second exits.
    #[test]
    fn a_live_pane_takes_two_requests_to_close() {
        let mut app = app_with_a_pane();
        assert!(!app.confirm_quit(), "first request must not exit");
        let msg = app.status.clone().map(|(m, _)| m).unwrap_or_default();
        assert!(msg.contains("1 pane open"), "no explanation given: {msg}");
        assert!(app.confirm_quit(), "second request should exit");
    }

    /// Nothing running → closing is immediate. The guard exists to protect live
    /// work, not to make an empty window argue with you.
    #[test]
    fn an_empty_window_closes_on_the_first_request() {
        let mut app = CrewApp::default();
        assert!(app.confirm_quit());
        assert!(app.status.is_none(), "an empty window should not prompt");
    }

    /// The prompt answers a keypress AND a click, so it must not name either
    /// input — "press quit again" is wrong when you clicked the close button.
    #[test]
    fn prompt_is_action_neutral_and_counts_panes() {
        let one = quit_prompt(1);
        assert!(one.contains("1 pane open"), "{one}");
        assert!(quit_prompt(3).contains("3 panes open"));
        for p in [one, quit_prompt(3)] {
            assert!(!p.contains("press"), "prompt names a keypress: {p}");
            assert!(!p.contains("click"), "prompt names a click: {p}");
        }
    }
}

//! An agent pane's turn ending while you are elsewhere says so, as a
//! terminal command finishing does: the "done" notification — the same
//! `notify_agent_done` switch and `notify_min_secs` threshold — and, when it
//! is not the pane you are in, its attention marker. (A turn you watched end
//! in the pane you are in needs no announcing.)
use crate::app::CrewApp;
use crate::notify::NotifyKind;

impl CrewApp {
    /// `turns`: the panes whose turn just ended, and how long each ran (ms).
    pub(crate) fn chat_turns_done(&mut self, turns: Vec<(usize, u64)>) {
        let here = self.win_focus.unwrap_or(true);
        for (i, ms) in turns {
            let watched = i == self.focused && here;
            if watched || ms < self.config.notify_min_secs.saturating_mul(1000) {
                continue;
            }
            let now = crate::anim::now_ms();
            let Some(p) = self.panes.get_mut(i) else {
                continue;
            };
            let title = p.title_text();
            if i != self.focused {
                crate::attention::raise(p, NotifyKind::AgentDone, now);
            }
            let took = crate::runclock::ladder(ms / 1000);
            self.notify(NotifyKind::AgentDone, title, format!("a {took} turn"));
        }
    }
}

#[cfg(test)]
#[path = "turndone_tests.rs"]
mod tests;

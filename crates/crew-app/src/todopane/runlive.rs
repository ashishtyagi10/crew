//! Whether the agent a todo was handed to is still at work — read off the
//! panes on the poll tick and handed to every todo pane as one set.
//!
//! The row's chip said `▶claude` forever after a run: while the agent
//! worked, after it exited, after its pane was closed. The pane knew
//! (`foreground_pid` is what the idle-shell rule reads; a chat pane knows
//! when it is busy) and the list did not ask. It asks here, and only while
//! some todo pane has a row that ran — a list with no runs costs no ioctl.
//! It hangs off `CrewApp` because a pane cannot see its siblings.
use std::collections::BTreeSet;

use crate::app::CrewApp;
use crate::pane::PaneContent;

impl CrewApp {
    /// The labels of every pane whose agent is running right now.
    fn busy_labels(&self) -> BTreeSet<String> {
        self.panes
            .iter()
            .filter(|p| match &p.content {
                PaneContent::Terminal(t) => t.pty.foreground_pid().is_some(),
                PaneContent::Chat(c) => c.is_busy(),
                _ => false,
            })
            .filter_map(|p| p.label.clone())
            .collect()
    }

    /// Refresh each todo pane's live set. Returns whether any changed — a
    /// redraw is due exactly when a chip flips.
    pub(crate) fn sync_todo_runs(&mut self) -> bool {
        let any_runs = self.panes.iter().any(|p| match &p.content {
            PaneContent::Todo(t) => t.items.iter().any(|it| it.run.is_some()),
            _ => false,
        });
        if !any_runs {
            return false;
        }
        let busy = self.busy_labels();
        let mut changed = false;
        for p in &mut self.panes {
            if let PaneContent::Todo(t) = &mut p.content {
                if t.live != busy {
                    t.live = busy.clone();
                    changed = true;
                }
            }
        }
        changed
    }
}

#[cfg(test)]
#[path = "runlive_tests.rs"]
mod tests;

//! Pane-management slash commands beyond the per-pane chords. `/only` closes
//! every pane except the focused one — a quick "focus mode", like tmux's
//! kill-other-panes / zellij's pane fullscreen-by-closing.
use crate::app::CrewApp;

impl CrewApp {
    /// Minimize the pane at `idx` into the left-nav PANES list (the `[-]` button
    /// on its border): it leaves the grid but keeps running; focusing it again
    /// (click its nav row, Cmd+N) restores it. Shows the nav when hidden — the
    /// pane minimizes *into* it.
    pub(crate) fn minimize_pane(&mut self, idx: usize) {
        if idx >= self.panes.len() {
            return;
        }
        let p = &self.panes[idx];
        self.ghosts.push(crate::ghost::Ghost::new(
            p.rect,
            p.title_text(),
            crate::ghost::Exit::Minimized,
            crate::anim::now_ms(),
        ));
        self.panes[idx].hidden = true;
        self.zoomed = false;
        if !self.config.show_nav {
            self.config.show_nav = true;
            self.config.save();
        }
        if idx == self.focused {
            // Focus the nearest visible pane; with none left, the input bar.
            match self.nearest_visible(idx) {
                Some(i) => self.focused = i,
                None => self.input.focused = true,
            }
        }
        self.set_status("minimized to nav — click its PANES row to restore");
        self.redraw();
    }

    /// Close all panes except the focused one. A no-op (with a hint) when there
    /// is one pane or none.
    /// `/pin` — keep the focused pane on the grid, or let it go again.
    ///
    /// The LRU is right about which pane you have not touched and wrong about
    /// whether that matters: the pane you are least likely to touch is often
    /// the agent you most want to keep watching.
    pub(crate) fn toggle_pin(&mut self) {
        let idx = self.focused;
        let on = !self.grid.is_pinned(idx);
        self.grid.set_pinned(idx, on);
        // The set of full tiles just changed; the same reconcile every other
        // promotion path goes through re-lays the grid.
        self.reconcile_grid();
        let title = self
            .panes
            .get(idx)
            .map(|p| p.title_text())
            .unwrap_or_default();
        self.set_status(match on {
            true => format!("{title} pinned to the grid"),
            false => format!("{title} unpinned"),
        });
    }

    pub(crate) fn close_other_panes(&mut self) {
        let others = self.panes.len().saturating_sub(1);
        if others > 0 && !self.pending.answered("only", std::time::Instant::now()) {
            let s = if others == 1 { "" } else { "s" };
            let ask = format!("close the other {others} pane{s}? /only again");
            self.pending.asking(&ask);
            self.set_status(ask);
            return;
        }
        if self.panes.len() <= 1 {
            self.set_status("only one pane");
            return;
        }
        let keep = self.focused.min(self.panes.len() - 1);
        self.panes.swap(0, keep);
        // `/only` closes many panes in one keystroke and does not go through
        // `close_pane`, so it records its own casualties — oldest first, so
        // repeated `/reopen` walks back up the grid in the order it fell.
        for p in &self.panes[1..] {
            self.closed.remember(p);
        }
        self.panes.truncate(1); // drops the rest (closing their PTYs)
        self.focused = 0;
        self.zoomed = false;
        self.input.focused = false;
        self.set_status("closed other panes");
        self.redraw();
    }

    /// Close every pane, returning to the welcome screen and input bar. A no-op
    /// (with a hint) when there are no panes.
    pub(crate) fn close_all_panes(&mut self) {
        if self.panes.is_empty() {
            self.set_status("no panes to close");
            return;
        }
        let n = self.panes.len();
        // A closed pane takes its scrollback, its running command and its
        // agent with it, and `/closeall` is one fuzzy keystroke from `/clear`.
        if !self.pending.answered("closeall", std::time::Instant::now()) {
            let ask = format!("close all {n} panes? /closeall again");
            self.pending.asking(&ask);
            self.set_status(ask);
            return;
        }
        // Reuse close_pane so the grid LRU and empty-state modes stay consistent.
        while !self.panes.is_empty() {
            self.close_pane(self.panes.len() - 1);
        }
        self.set_status(format!("closed {n} panes"));
        self.redraw();
    }
}

#[cfg(test)]
#[path = "panemanage_tests.rs"]
mod tests;

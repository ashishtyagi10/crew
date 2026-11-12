//! The app's side of [`crate::tabs`]: opening, switching, renaming and
//! moving panes between tabs, and the once-a-frame bookkeeping that keeps
//! the panes, the grid and the tab list agreeing.
use crate::app::CrewApp;

impl CrewApp {
    /// Whether pane `i` lives in the tab on screen. A pane not yet adopted
    /// (`tab == 0`, pushed since the last frame) belongs to it.
    pub(crate) fn in_active_tab(&self, i: usize) -> bool {
        self.panes
            .get(i)
            .is_some_and(|p| p.tab == 0 || p.tab == self.tabs.active)
    }

    /// The frame's bookkeeping, first thing in `reconcile_grid`: new panes
    /// join the tab on screen, focus on a pane in another tab brings that tab
    /// forward, and a tab with no panes left goes.
    pub(crate) fn sync_tabs(&mut self) {
        let active = self.tabs.active;
        for p in &mut self.panes {
            if p.tab == 0 || self.tabs.pos(p.tab).is_none() {
                p.tab = active;
            }
        }
        if !self.input.focused {
            if let Some(t) = self.panes.get(self.focused).map(|p| p.tab) {
                self.switch_grid(t);
            }
        }
        let used: Vec<u32> = self.panes.iter().map(|p| p.tab).collect();
        if let Some(to) = self.tabs.prune(&used) {
            self.switch_grid(to);
            self.focus_in_tab();
        }
    }

    /// Put the active tab's grid away and take out `to`'s. Focus is the
    /// caller's business.
    fn switch_grid(&mut self, to: u32) {
        if to == self.tabs.active || self.tabs.pos(to).is_none() {
            return;
        }
        let cur = std::mem::take(&mut self.grid);
        if let Some(t) = self.tabs.get_mut(self.tabs.active) {
            t.grid = cur;
        }
        self.grid = self
            .tabs
            .get_mut(to)
            .map(|t| std::mem::take(&mut t.grid))
            .unwrap_or_default();
        self.tabs.active = to;
        // The tab's cards assemble as they arrive, the way a new pane's do.
        let now = crate::anim::now_ms();
        for p in self.panes.iter_mut().filter(|p| p.tab == to) {
            p.born_ms = now;
        }
    }

    /// Focus the active tab's last-used visible pane; with every one of its
    /// panes minimized, the input bar.
    fn focus_in_tab(&mut self) {
        let shown = |i: usize| {
            self.panes
                .get(i)
                .is_some_and(|p| p.tab == self.tabs.active && !p.hidden)
        };
        let pick = (self.grid.recent().iter().copied().find(|&i| shown(i)))
            .or_else(|| (0..self.panes.len()).find(|&i| shown(i)));
        match pick {
            Some(i) => {
                self.focused = i;
                self.input.focused = false;
            }
            None => {
                let first = (0..self.panes.len()).find(|&i| self.in_active_tab(i));
                self.focused = first.unwrap_or(self.focused);
                self.input.focused = true;
            }
        }
    }

    /// Bring tab `id` on screen, on the pane it was left at.
    pub(crate) fn activate_tab(&mut self, id: u32) {
        if id == self.tabs.active {
            return;
        }
        self.switch_grid(id);
        self.focus_in_tab();
        self.redraw();
    }

    /// Cmd+Shift+] / [ — the next or previous tab, wrapping.
    pub(crate) fn step_tab(&mut self, dir: i32) {
        match self.tabs.step(dir) {
            Some(id) => self.activate_tab(id),
            None => self.set_status("one tab open \u{b7} Cmd+T opens another"),
        }
    }

    /// Cmd+Opt+N — tab N, as the nav numbers them.
    pub(crate) fn goto_tab(&mut self, n: usize) {
        match self.tabs.id_at(n) {
            Some(id) => self.activate_tab(id),
            None => self.set_status(format!("no tab {n}")),
        }
    }

    /// Cmd+T — a new tab holding one fresh shell. With the tab on screen
    /// still empty (the welcome screen) the shell simply opens there.
    pub(crate) fn new_tab(&mut self) {
        self.own_tab();
        self.spawn_new_pane();
    }

    /// Bring a fresh tab on screen for the pane about to be opened, unless
    /// the one on screen is still empty. A spawn that then fails leaves no
    /// trace: `sync_tabs` drops a tab with nothing in it.
    pub(crate) fn own_tab(&mut self) {
        if (0..self.panes.len()).any(|i| self.in_active_tab(i)) {
            let id = self.tabs.add();
            self.switch_grid(id);
        }
    }

    /// `/far`: the file manager wants the whole window, so it opens in a
    /// tab of its own. (A restored Far pane goes back to the tab it was in.)
    pub(crate) fn far_in_own_tab(&mut self) {
        self.own_tab();
        self.spawn_far_pane();
    }

    /// What the nav calls tab `id`: its `/tab rename` name, else the title of
    /// the first pane in it.
    pub(crate) fn tab_title(&self, id: u32) -> String {
        if let Some(n) = self.tabs.name(id) {
            return n.to_string();
        }
        let first = self.panes.iter().find(|p| p.tab == id);
        first.map_or_else(|| "empty".into(), |p| p.title_text())
    }

    /// Send the focused pane to tab `n` (1-based; one past the last opens a
    /// new tab) and follow it there.
    pub(crate) fn move_pane_to_tab(&mut self, n: usize) {
        let Some(i) = (!self.panes.is_empty()).then_some(self.focused) else {
            return self.set_status("no pane to move");
        };
        let id = match self.tabs.id_at(n) {
            Some(id) => id,
            None if n == self.tabs.list.len() + 1 => self.tabs.push(),
            None => return self.set_status(format!("no tab {n}")),
        };
        if self.panes[i].tab == id {
            return self.set_status(format!("already in tab {n}"));
        }
        self.panes[i].tab = id;
        self.switch_grid(id);
        self.focused = i;
        self.input.focused = false;
        self.set_status(format!("moved to tab {n}"));
    }
}

#[cfg(test)]
#[path = "owntab_tests.rs"]
mod own_tests;
#[cfg(test)]
#[path = "tabsapp_tests.rs"]
mod tests;

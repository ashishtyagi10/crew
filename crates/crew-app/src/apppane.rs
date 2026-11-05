//! Panes coming and going: closing one, deciding what gets focus next, and
//! packing what remains back into the near-square grid.
//!
//! Split out of [`crate::app`] for the line cap.
use crate::app::{CrewApp, FALLBACK_SIZE};
use crate::session::grid_for;
use crew_render::Renderer;
use crew_term::GridSize;

impl CrewApp {
    pub(crate) fn current_grid(renderer: &Renderer) -> GridSize {
        let (cell_w, cell_h) = renderer.cell_size();
        if cell_w > 0.0 && cell_h > 0.0 {
            let (sw, sh) = renderer.surface_size();
            grid_for(sw, sh, cell_w, cell_h)
        } else {
            FALLBACK_SIZE
        }
    }

    /// Close pane at `idx`.  Returns `true` if the app should exit.
    pub fn close_pane(&mut self, idx: usize) -> bool {
        // Closing the pane that had the screen lands on the grid you left.
        let had_screen = self.zoomed_pane() == Some(idx);
        if idx < self.panes.len() {
            // Record where the card was before the pane is gone: everything
            // downstream reads `panes`, so the pane cannot linger — only the
            // frame it leaves behind can.
            let p = &self.panes[idx];
            self.ghosts.push(crate::ghost::Ghost::new(
                p.rect,
                p.title_text(),
                crate::ghost::Exit::Closed,
                crate::anim::now_ms(),
            ));
            // Everything a reopen needs is read off the live pane, so it has
            // to be read HERE — one line later the PTY is being reaped and
            // the directory it was standing in is gone with it.
            self.closed.remember(p);
            self.panes.remove(idx);
            self.grid.on_close(idx);
            self.tabs.on_close(idx);
        }
        if self.panes.is_empty() {
            // No panel selected → focus returns to the input bar; reset modes.
            self.focused = 0;
            self.input.focused = true;
            self.broadcast = false;
            self.input.broadcast = false;
            return false;
        }
        if had_screen && self.focus_grid() {
            return false;
        }
        // The tab's last pane gone: the tab goes, and its neighbour comes on.
        if !(0..self.panes.len()).any(|i| self.in_active_tab(i)) {
            self.sync_tabs();
            return false;
        }
        self.focused = self.focused.min(self.panes.len() - 1);
        // Never let the clamp land focus on a pane minimized into the nav, or
        // in another tab — reconcile_grid would silently bring it forward.
        // Prefer a visible pane here; with none, the input bar takes focus.
        if self.panes[self.focused].hidden || !self.in_active_tab(self.focused) {
            match self.nearest_visible(self.focused) {
                Some(i) => self.focused = i,
                None => self.input.focused = true,
            }
        }
        false
    }

    /// The non-hidden pane index nearest to `idx`, if any pane is visible.
    pub(crate) fn nearest_visible(&self, idx: usize) -> Option<usize> {
        (0..self.panes.len())
            .filter(|&i| !self.panes[i].hidden && self.in_active_tab(i))
            .min_by_key(|&i| i.abs_diff(idx))
    }

    /// Keep the grid LRU in step with `self.panes` and the current focus. Adds
    /// any visible pane index not yet tracked (newly spawned), drops hidden and
    /// stale indices, and marks the focused pane most-recently-active. Called
    /// once per frame from `build_frame`.
    pub(crate) fn reconcile_grid(&mut self) {
        self.sync_tabs();
        let n = self.panes.len();
        // Keyboard-focusing a hidden pane restores it — the one rule that makes
        // every focus path (nav-row click, Cmd+N, spawn) a restore path. The
        // input bar holding focus means no pane is active, so nothing restores.
        if !self.input.focused {
            if let Some(p) = self.panes.get_mut(self.focused) {
                // Restoring re-stamps the birth clock, so a pane coming back
                // out of the nav assembles exactly as a new one does — it is,
                // as far as the grid is concerned, arriving.
                if p.hidden {
                    p.born_ms = crate::anim::now_ms();
                }
                p.hidden = false;
            }
        }
        // Hidden panes leave the grid without reindexing — a hide keeps the
        // panes vec intact, unlike a close — and so do zoomed ones, which have
        // a screen of their own (`panezoom`). Also drops any stale index past
        // the end (defensive; close_pane already fixes the common case).
        let (panes, tab) = (&self.panes, self.tabs.active);
        self.grid.retain(|i| {
            panes
                .get(i)
                .is_some_and(|p| !p.hidden && !p.zoomed() && (p.tab == tab || p.tab == 0))
        });
        for idx in 0..n {
            if self.on_grid(idx)
                && !self.grid.full().contains(&idx)
                && !self.grid.minimized().contains(&idx)
            {
                self.grid.add(idx);
            }
        }
        if n > 0 && self.in_active_tab(self.focused.min(n - 1)) {
            self.grid.touch(self.focused.min(n - 1));
        }
    }

    /// Focus the most-recently-pushed pane and move keyboard focus off the input bar.
    pub(crate) fn focus_new_pane(&mut self) {
        self.focused = self.panes.len().saturating_sub(1);
        self.input.focused = false;
    }

    /// Toggle the window's maximized state and persist it.
    pub(crate) fn toggle_maximize(&mut self) {
        if let Some(w) = &self.window {
            let m = !w.is_maximized();
            w.set_maximized(m);
            self.config.maximized = m;
        }
        self.config.save();
    }

    pub(crate) fn toggle_sidebar(&mut self) {
        self.config.show_nav = !self.config.show_nav;
        self.config.save();
        self.redraw();
    }

    /// Open the nav, or put it back to the rail. What the chevron does.
    pub(crate) fn toggle_nav_collapsed(&mut self) {
        self.config.nav_collapsed = !self.config.nav_collapsed;
        self.config.save();
        self.redraw();
    }
}

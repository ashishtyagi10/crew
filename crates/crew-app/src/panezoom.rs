//! Which pane gets the whole content area, and how you move between them.
//!
//! Zoom used to be one flag on the app — "the focused pane fills the window"
//! — so it belonged to whichever pane had focus. `/far` set it on the way
//! in, and the first Cmd+2 handed the whole window to a shell; Cmd+Z back
//! out, and far came back as a quarter tile. The inbox never set it at all.
//!
//! Now it is the pane's. Some kinds are places you go rather than tiles you
//! keep beside the work — the inbox, far, the settings form, the viewer —
//! and those take the whole content area whenever they have focus and never
//! sit on the grid. Everything else is a tile. Cmd+Z flips either, and the
//! flip stays with that pane: a docked inbox is a tile until you zoom it
//! again, and a zoomed shell keeps its screen while you look at the grid.
//!
//! So the window is a row of SCREENS — the grid, then each zoomed pane — and
//! Cmd+Opt+←/→ (Ctrl+Alt off a Mac) steps along it.
use crate::app::CrewApp;
use crate::pane::{Pane, PaneContent};

impl PaneContent {
    /// Whether this kind takes the whole content area unless Cmd+Z says not.
    pub fn zooms(&self) -> bool {
        matches!(
            self,
            Self::Inbox(_) | Self::Far(_) | Self::Settings(_) | Self::View(_)
        )
    }
}

impl Pane {
    /// Whether this pane takes the whole content area when it has focus:
    /// its own Cmd+Z, else its kind's default.
    pub fn zoomed(&self) -> bool {
        self.zoom.unwrap_or_else(|| self.content.zooms())
    }
}

impl CrewApp {
    /// The pane drawn alone over the content area, if one is: the focused
    /// pane, when it is zoomed. Never a minimized one — with every pane put
    /// away, focus can still name one while the input bar holds the keys.
    pub(crate) fn zoomed_pane(&self) -> Option<usize> {
        let i = self.focused.min(self.panes.len().checked_sub(1)?);
        let p = &self.panes[i];
        (p.zoomed() && !p.hidden).then_some(i)
    }

    /// Whether one pane fills the content area, rather than the grid.
    pub(crate) fn zoomed(&self) -> bool {
        self.zoomed_pane().is_some()
    }

    /// Whether pane `i` sits on the grid: neither minimized nor zoomed.
    pub(crate) fn on_grid(&self, i: usize) -> bool {
        self.panes.get(i).is_some_and(|p| !p.hidden && !p.zoomed())
    }

    /// Back to the grid, on the tile used last. `false` when the grid is
    /// empty, and nothing moves.
    pub(crate) fn focus_grid(&mut self) -> bool {
        let last = self
            .grid
            .recent()
            .iter()
            .copied()
            .find(|&i| self.on_grid(i));
        let Some(i) = last.or_else(|| (0..self.panes.len()).find(|&i| self.on_grid(i))) else {
            return false;
        };
        self.focused = i;
        self.input.focused = false;
        true
    }

    /// Cmd+Opt+→ (`dir` 1) or ← (-1): the next screen along — the grid,
    /// then each zoomed pane in pane order, wrapping. A minimized zoomed pane
    /// is still a screen; landing on it brings it back (`reconcile_grid`).
    pub(crate) fn step_screen(&mut self, dir: i32) {
        let grid = (0..self.panes.len()).any(|i| self.on_grid(i));
        let screens: Vec<Option<usize>> = grid
            .then_some(None)
            .into_iter()
            .chain(
                (0..self.panes.len())
                    .filter(|&i| self.panes[i].zoomed())
                    .map(Some),
            )
            .collect();
        let here = screens.iter().position(|&s| s == self.zoomed_pane());
        let n = screens.len() as i32;
        let to = match here {
            Some(_) if n < 2 => {
                let note = "one screen open \u{b7} /inbox, /far and Cmd+Z each open another";
                return self.set_status(crate::chordglyph::prose(note).into_owned());
            }
            Some(at) => screens[(at as i32 + dir).rem_euclid(n) as usize],
            // On no screen at all: every pane minimized, the bar holding focus.
            None => {
                let end = if dir < 0 {
                    screens.last()
                } else {
                    screens.first()
                };
                let Some(&s) = end else {
                    return;
                };
                s
            }
        };
        match to {
            Some(i) => {
                self.focused = i;
                self.input.focused = false;
            }
            None => drop(self.focus_grid()),
        }
    }

    /// Esc on the inbox with nothing left to cancel: out of sight, still
    /// open, so coming back does not fetch the mail again. Zoomed, that is
    /// back to the grid; docked on it, or with no grid to go back to, into
    /// the nav — exactly what Esc does to the todo list.
    pub(crate) fn put_away(&mut self, i: usize) {
        let Some((zoomed, title)) = self.panes.get(i).map(|p| (p.zoomed(), p.title_text())) else {
            return;
        };
        if !(zoomed && self.focus_grid()) {
            return self.minimize_pane(i);
        }
        let note = format!("{title} still open \u{b7} Cmd+Opt+\u{2190}\u{2192} to come back");
        self.set_status(crate::chordglyph::prose(&note).into_owned());
    }
}

#[cfg(test)]
#[path = "panezoom_tests.rs"]
mod tests;

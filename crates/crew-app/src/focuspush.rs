//! Tells terminal programs that asked (DECSET 1004) when their pane gains or
//! loses keyboard focus: neovim's `FocusGained` reload, tmux's focus events,
//! TUIs that pause or dim when you look away. Focus moves at a score of sites
//! (chords, clicks, spawns, closes, the input bar, the window itself), so
//! rather than hook each one this runs once per poll tick and lets crew-term
//! diff each pane's answer against what its program was last told.
use crate::app::CrewApp;
use crate::pane::PaneContent;

/// Whether pane `i` holds the keyboard: the window has focus (unknown counts
/// as yes, as everywhere else), `i` is the focused pane, and the input bar is
/// not taking the keys instead.
pub(crate) fn has_keys(i: usize, focused: usize, win: Option<bool>, bar: bool) -> bool {
    win.unwrap_or(true) && i == focused && !bar
}

impl CrewApp {
    /// Per poll tick: report each terminal pane's focus to its program when
    /// it changed. Returns how many reports were written (the tests read it).
    pub(crate) fn push_focus(&mut self) -> usize {
        let (focused, win, bar) = (self.focused, self.win_focus, self.input.focused);
        let mut wrote = 0;
        for (i, p) in self.panes.iter_mut().enumerate() {
            if let PaneContent::Terminal(t) = &mut p.content {
                wrote += usize::from(t.pty.report_focus(has_keys(i, focused, win, bar)));
            }
        }
        wrote
    }
}

#[cfg(test)]
#[path = "focuspush_tests.rs"]
mod tests;

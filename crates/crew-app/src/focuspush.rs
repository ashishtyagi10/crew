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
    /// A find bar belongs to the pane that has the keys: once focus moves to
    /// another pane or the input bar it closes, so it neither keeps drawing
    /// on an unfocused pane nor silently eats keys when focus comes back.
    /// (The window losing focus to another app keeps it.) Run per tick and
    /// before a key is routed, so a click-then-type never beats the sweep.
    pub(crate) fn close_find_off_focus(&mut self) {
        let (focused, bar) = (self.focused, self.input.focused);
        for (i, p) in self.panes.iter_mut().enumerate() {
            if let PaneContent::Terminal(t) = &mut p.content {
                if t.find.is_some() && (i != focused || bar) {
                    t.find = None;
                }
            }
        }
    }

    /// Per poll tick: report each terminal pane's focus to its program when
    /// it changed. Returns how many reports were written (the tests read it).
    pub(crate) fn push_focus(&mut self) -> usize {
        self.close_find_off_focus();
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

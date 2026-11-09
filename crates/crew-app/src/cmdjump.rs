//! Cmd+Opt+↑/↓ (Ctrl+Alt+↑/↓ off a Mac): scroll a terminal pane to the
//! start of the previous / next command you ran in it.
//!
//! Crew already knows where each command began ([`crate::cmdspan`]) and ticks
//! those rows on the pane's left border, but reaching one meant wheeling
//! through a long build or agent run by eye. Terminal.app, iTerm2, Ghostty
//! and WezTerm all give this a key. Theirs is Cmd+↑/↓, which crew spends on
//! walking the grid, so it rides on the screens' chord instead: Opt+←/→
//! steps the screens sideways, Opt+↑/↓ steps the commands up and down.
//!
//! A start is "landed on" when it is the window's top row, which is where
//! the jump puts it, so the next press goes past it rather than to it again.
use crate::app::CrewApp;
use crate::pane::PaneContent;

impl CrewApp {
    /// Move the focused terminal's view to the previous (`up`) or next
    /// command start. `false` when no terminal is focused, so the arrow keeps
    /// its old meaning there; `true` (the key is spent) when there is simply
    /// nowhere further to go.
    pub(crate) fn jump_command(&mut self, up: bool) -> bool {
        let Some(pane) = self.panes.get_mut(self.focused) else {
            return false;
        };
        let rows = usize::from(pane.grid.rows);
        let PaneContent::Terminal(t) = &mut pane.content else {
            return false;
        };
        let now = t.pty.lines_seen();
        let off = t.pty.display_offset();
        let history = t.pty.history_lines();
        let starts: Vec<usize> = t.spans.recent().map(|s| s.from).collect();
        if let Some(to) = jump_target(&starts, now, rows, history, off, up) {
            t.pty.scroll(to as i32 - off as i32);
        }
        true
    }
}

/// The display offset that puts the previous (`up`) or next command start on
/// the top row of a `rows`-row window, from the current offset `off`, after
/// `now` lines in all (`lines_seen`) with `history` of them still above the
/// screen. A start too near the bottom to reach the top row lands at offset 0
/// (the live screen). A start deeper than the history has been thrown away
/// with the scrollback: it is skipped, not clamped to the top row, which
/// would show some later line and call it the command. Going down past the
/// newest start returns to the bottom; `None` when the view is already as
/// far as it can go.
pub(crate) fn jump_target(
    starts: &[usize],
    now: usize,
    rows: usize,
    history: usize,
    off: usize,
    up: bool,
) -> Option<usize> {
    let at = |from: usize| now.saturating_sub(rows).saturating_sub(from);
    let offsets = starts
        .iter()
        .map(|&from| at(from))
        .filter(|&o| o <= history);
    let to = match up {
        true => offsets.filter(|&o| o > off).min(),
        false => offsets.filter(|&o| o < off).max().or(Some(0)),
    };
    to.filter(|&o| o != off)
}

#[cfg(test)]
#[path = "cmdjump_tests.rs"]
mod tests;

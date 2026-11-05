//! The walk behind `/find` and `/errors`: one pass up the focused terminal's
//! scrollback for the newest matching line, then the view moved to show it.
//!
//! It used to scroll the view a line at a time and rebuild the whole screen
//! of cells twice per line on the window's thread: 160 ms for a match 5,000
//! lines back, and a miss for anything older though the scrollback holds
//! 10,000. Now the terminal reads its own grid (`find_back`) and the view
//! moves once.
use crate::app::CrewApp;
use crate::pane::PaneContent;
use crew_term::{RenderCell, TermModel};

impl CrewApp {
    /// Walk the focused terminal's scrollback up from the view's bottom row
    /// (from the row above the last hit when `repeat`) for the newest line
    /// `hit` accepts, and bring it into view. `None` when no terminal is
    /// focused, else whether a line was found.
    pub(crate) fn seek_back(
        &mut self,
        repeat: bool,
        hit: &mut dyn FnMut(&str) -> bool,
    ) -> Option<bool> {
        let pane = self.panes.get_mut(self.focused)?;
        let rows = pane.grid.rows as usize;
        let PaneContent::Terminal(t) = &mut pane.content else {
            return None;
        };
        let off = t.pty.display_offset();
        let from = match self.find_hit {
            Some(h) if repeat => off + h + 1,
            _ => off,
        };
        let Some(depth) = t.pty.find_back(from, hit) else {
            return Some(false);
        };
        let to = view_for(depth, off, rows, t.pty.history_lines());
        t.pty.scroll(to as i32 - off as i32);
        self.find_hit = Some(depth - to);
        Some(true)
    }

    /// `f` over the focused terminal's visible cells (0 when it isn't one).
    pub(crate) fn in_view(&self, f: impl Fn(&[RenderCell], u16, u16) -> usize) -> usize {
        match self.panes.get(self.focused).map(|p| (&p.content, p.grid)) {
            Some((PaneContent::Terminal(t), g)) => f(&t.pty.cells(false), g.cols, g.rows),
            _ => 0,
        }
    }
}

/// The display offset that shows a hit `depth` lines above the screen's
/// bottom row: the current `off` when the hit is already in view, else the
/// one that puts it mid-view, clamped to the `history` there is. A hit on
/// the top row read as the edge of the screen, not as the answer.
pub(crate) fn view_for(depth: usize, off: usize, rows: usize, history: usize) -> usize {
    if depth >= off && depth < off + rows {
        return off;
    }
    depth
        .saturating_sub(rows.saturating_sub(1) / 2)
        .min(history)
}

/// Non-overlapping occurrences of the smart-case `needle` (see [`needle`])
/// in `line`, folded the way [`rows_text`] folds the screen.
pub(crate) fn matches_in(line: &str, needle: &str, ci: bool) -> usize {
    if needle.is_empty() {
        return 0;
    }
    if ci {
        line.to_ascii_lowercase().matches(needle).count()
    } else {
        line.matches(needle).count()
    }
}

#[cfg(test)]
#[path = "findback_tests.rs"]
mod tests;

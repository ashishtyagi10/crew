//! Scrollback search: `/find <term>` brings the focused terminal's most
//! recent line containing `term` into view, and `/errors` its latest error.
use crate::app::CrewApp;
use crate::errscan;
use crate::findback::matches_in;
use crate::pane::PaneContent;
use crew_term::{RenderCell, TermModel};

/// Build the grid's rows as strings in a single pass over `cells` (smart-case:
/// lowercased when `ci`), for what is in view: the scrollback walk reads the
/// grid itself (`find_back`).
pub(crate) fn rows_text(cells: &[RenderCell], cols: u16, rows: u16, ci: bool) -> Vec<String> {
    let mut lines = vec![vec![' '; cols as usize]; rows as usize];
    for c in cells {
        if (c.row as usize) < lines.len() && (c.col as usize) < cols as usize {
            lines[c.row as usize][c.col as usize] = if ci { c.c.to_ascii_lowercase() } else { c.c };
        }
    }
    lines.into_iter().map(|l| l.into_iter().collect()).collect()
}

/// The smart-case needle for `term`: lowercased unless `term` has an uppercase
/// letter (in which case the match is case-sensitive). Returns `(needle, ci)`.
pub(crate) fn needle(term: &str) -> (String, bool) {
    let ci = !term.chars().any(|c| c.is_uppercase());
    let n = if ci {
        term.to_lowercase()
    } else {
        term.to_string()
    };
    (n, ci)
}

/// Count non-overlapping occurrences of `term` across the `cols × rows` grid
/// (smart-case, see [`needle`]) — the matches visible on screen.
pub(crate) fn count_in_grid(cells: &[RenderCell], term: &str, cols: u16, rows: u16) -> usize {
    if term.is_empty() {
        return 0;
    }
    let (needle, ci) = needle(term);
    rows_text(cells, cols, rows, ci)
        .iter()
        .map(|line| line.matches(needle.as_str()).count())
        .sum()
}

impl CrewApp {
    /// Clear the focused terminal's scrollback (CSI 3 J), keeping the visible
    /// screen, and snap back to the live bottom.
    pub(crate) fn clear_focused_scrollback(&mut self) {
        let mut cleared = false;
        if let Some(pane) = self.panes.get_mut(self.focused) {
            if let PaneContent::Terminal(t) = &mut pane.content {
                t.pty.feed(b"\x1b[3J");
                t.pty.scroll_to_bottom();
                cleared = true;
            }
        }
        self.set_status(if cleared {
            "scrollback cleared"
        } else {
            "nothing to clear"
        });
    }

    /// Clear every terminal pane's scrollback, snapping each to its live bottom.
    pub(crate) fn clear_all_scrollback(&mut self) {
        let mut n = 0;
        for pane in &mut self.panes {
            if let PaneContent::Terminal(t) = &mut pane.content {
                t.pty.feed(b"\x1b[3J");
                t.pty.scroll_to_bottom();
                n += 1;
            }
        }
        if n > 0 {
            self.set_status(format!("cleared {}", crate::wording::count(n, "pane")));
        } else {
            self.set_status("nothing to clear");
        }
    }

    /// Bring the focused terminal's most recent line containing `term` into
    /// view (the one above the last hit when repeating). Always repaints,
    /// and flashes a status when there's no match.
    pub(crate) fn find_in_terminal(&mut self, term: &str) {
        if term.is_empty() {
            return self.set_status("usage: /find <text> \u{2014} or /find all <text>");
        }
        // Repeating the same term continues upward from the current match.
        let repeat = self.last_find.as_deref() == Some(term);
        self.last_find = Some(term.to_string());
        let (needle, ci) = needle(term);
        let Some(found) = self.seek_back(repeat, &mut |l| matches_in(l, &needle, ci) > 0) else {
            return;
        };
        if !found {
            let older = if repeat { "older " } else { "" };
            return self.set_status(format!("no {older}match for \u{201c}{term}\u{201d}"));
        }
        // At least the hit: one a soft wrap splits is not counted row by row.
        let count = self
            .in_view(|cells, c, r| count_in_grid(cells, term, c, r))
            .max(1);
        let n = crate::wording::count_as(count, "match", "matches");
        self.set_status(format!("{n} for \u{201c}{term}\u{201d} in view"));
        self.redraw();
    }

    /// `/errors`: bring the focused terminal's most recent line that reads as
    /// an error ([`crate::errscan`]) into view, and say how many are in view.
    ///
    /// Repeating it steps further back, the way a repeated `/find` does — a long
    /// build has more than one failure, and the one you want is rarely the last.
    pub(crate) fn find_error_in_terminal(&mut self) {
        let repeat = self.last_find.as_deref() == Some(ERRORS);
        self.last_find = Some(ERRORS.to_string());
        let Some(found) = self.seek_back(repeat, &mut |l| errscan::looks_like_error(l)) else {
            return self.set_status("no terminal pane focused");
        };
        if !found {
            return self.set_status("no errors in this pane");
        }
        let errs = |cells: &[RenderCell], c, r| {
            let lines = rows_text(cells, c, r, false);
            lines
                .iter()
                .filter(|l| errscan::looks_like_error(l))
                .count()
        };
        match self.in_view(errs).max(1) {
            1 => self.set_status("1 error in view"),
            n => self.set_status(format!("{n} errors in view")),
        }
        self.redraw();
    }
}

/// The `last_find` sentinel for an error walk, so repeating `/errors` steps
/// back rather than starting over. It cannot collide with a real search term:
/// `/find` refuses an empty one, and this is not text anyone can type.
const ERRORS: &str = "\u{0}errors";

#[cfg(test)]
#[path = "search_tests.rs"]
mod tests;

//! Scrollback search: `/find <term>` scrolls the focused terminal back to the
//! most recent line containing `term`.
use crate::app::CrewApp;
use crate::errscan;
use crate::pane::PaneContent;
use crew_term::{RenderCell, TermModel};

/// Safety bound on how many lines a single search scrolls through.
const MAX_STEPS: usize = 5000;

/// Build the grid's rows as strings in a single pass over `cells` (smart-case:
/// lowercased when `ci`). Avoids rescanning every cell per row — the per-step
/// cost of the `/find` scroll loop drops from O(rows·cells) to O(cells).
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

/// Whether any row of the `cols × rows` grid (rebuilt from `cells`) contains
/// `term`, matched with smart case.
pub(crate) fn grid_contains(cells: &[RenderCell], term: &str, cols: u16, rows: u16) -> bool {
    if term.is_empty() {
        return false;
    }
    let (needle, ci) = needle(term);
    rows_text(cells, cols, rows, ci)
        .iter()
        .any(|line| line.contains(needle.as_str()))
}

/// Count non-overlapping occurrences of `term` across the `cols × rows` grid
/// (smart-case, same rule as [`grid_contains`]) — the matches visible on screen.
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
            self.set_status(format!("cleared {n} panes"));
        } else {
            self.set_status("nothing to clear");
        }
    }

    /// Scroll the focused terminal back to the most recent line containing
    /// `term` (stops at the current view, or the top of the scrollback). Always
    /// repaints, and flashes a status when there's no match.
    pub(crate) fn find_in_terminal(&mut self, term: &str) {
        if term.is_empty() {
            return;
        }
        // Repeating the same term continues upward from the current match.
        let repeat = self.last_find.as_deref() == Some(term);
        self.last_find = Some(term.to_string());
        let focused = self.focused;
        let mut searched = false;
        let mut found = false;
        let mut count = 0;
        if let Some(pane) = self.panes.get_mut(focused) {
            let (cols, rows) = (pane.grid.cols, pane.grid.rows);
            if let PaneContent::Terminal(t) = &mut pane.content {
                searched = true;
                if repeat {
                    t.pty.scroll(1); // step past the current match
                }
                for _ in 0..MAX_STEPS {
                    if grid_contains(&t.pty.cells(false), term, cols, rows) {
                        found = true;
                        count = count_in_grid(&t.pty.cells(false), term, cols, rows);
                        break;
                    }
                    let before = t.pty.display_offset();
                    t.pty.scroll(1);
                    if t.pty.display_offset() == before {
                        break; // reached the top of the scrollback
                    }
                }
            }
        }
        // Repaint regardless (the old code skipped redraw on a hit, so the match
        // scroll never showed); report the in-view match count, or a miss.
        if searched {
            if found {
                let plural = if count == 1 { "" } else { "es" };
                self.set_status(format!("{count} match{plural} for '{term}' in view"));
                self.redraw();
            } else {
                self.set_status(format!("no match for '{term}'"));
            }
        }
    }
}

/// `/errors`: scroll the focused terminal back to the most recent line that
/// reads as an error ([`crate::errscan`]), and say how many are in view.
///
/// Repeating it steps further back, the way a repeated `/find` does — a long
/// build has more than one failure, and the one you want is rarely the last.
impl CrewApp {
    pub(crate) fn find_error_in_terminal(&mut self) {
        let repeat = self.last_find.as_deref() == Some(ERRORS);
        self.last_find = Some(ERRORS.to_string());
        let focused = self.focused;
        let mut searched = false;
        let mut found = 0usize;
        if let Some(pane) = self.panes.get_mut(focused) {
            let (cols, rows) = (pane.grid.cols, pane.grid.rows);
            if let PaneContent::Terminal(t) = &mut pane.content {
                searched = true;
                if repeat {
                    t.pty.scroll(1);
                }
                for _ in 0..MAX_STEPS {
                    let lines = rows_text(&t.pty.cells(false), cols, rows, false);
                    let n = lines
                        .iter()
                        .filter(|l| errscan::looks_like_error(l))
                        .count();
                    if n > 0 {
                        found = n;
                        break;
                    }
                    let before = t.pty.display_offset();
                    t.pty.scroll(1);
                    if t.pty.display_offset() == before {
                        break;
                    }
                }
            }
        }
        if !searched {
            self.set_status("no terminal pane focused");
            return;
        }
        match found {
            0 => self.set_status("no errors in this pane"),
            1 => {
                self.set_status("1 error in view");
                self.redraw();
            }
            n => {
                self.set_status(format!("{n} errors in view"));
                self.redraw();
            }
        }
    }
}

/// The `last_find` sentinel for an error walk, so repeating `/errors` steps
/// back rather than starting over. It cannot collide with a real search term:
/// `/find` refuses an empty one, and this is not text anyone can type.
const ERRORS: &str = "\u{0}errors";

#[cfg(test)]
#[path = "search_tests.rs"]
mod tests;

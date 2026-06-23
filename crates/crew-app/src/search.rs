//! Scrollback search: `/find <term>` scrolls the focused terminal back to the
//! most recent line containing `term`.
use crate::app::CrewApp;
use crate::pane::PaneContent;
use crew_term::{RenderCell, TermModel};

/// Safety bound on how many lines a single search scrolls through.
const MAX_STEPS: usize = 5000;

/// Build the grid's rows as strings in a single pass over `cells` (smart-case:
/// lowercased when `ci`). Avoids rescanning every cell per row — the per-step
/// cost of the `/find` scroll loop drops from O(rows·cells) to O(cells).
fn rows_text(cells: &[RenderCell], cols: u16, rows: u16, ci: bool) -> Vec<String> {
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
fn needle(term: &str) -> (String, bool) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(col: u16, row: u16, c: char) -> RenderCell {
        RenderCell {
            col,
            row,
            c,
            fg: (0, 0, 0),
            bg: (0, 0, 0),
            bold: false,
            italic: false,
        }
    }

    #[test]
    fn grid_contains_finds_substring_in_a_row() {
        // "hi" on row 1.
        let cells = [cell(0, 1, 'h'), cell(1, 1, 'i')];
        assert!(grid_contains(&cells, "hi", 10, 3));
        assert!(!grid_contains(&cells, "bye", 10, 3));
        assert!(!grid_contains(&cells, "", 10, 3));
    }

    #[test]
    fn grid_contains_smart_case() {
        // "Hello" on row 0.
        let cells = [
            cell(0, 0, 'H'),
            cell(1, 0, 'e'),
            cell(2, 0, 'l'),
            cell(3, 0, 'l'),
            cell(4, 0, 'o'),
        ];
        // all-lowercase term → case-insensitive, matches.
        assert!(grid_contains(&cells, "hello", 10, 1));
        assert!(grid_contains(&cells, "ell", 10, 1));
        // a term with an uppercase letter → case-sensitive.
        assert!(grid_contains(&cells, "Hello", 10, 1));
        assert!(!grid_contains(&cells, "HELLO", 10, 1));
    }

    #[test]
    fn count_in_grid_counts_all_occurrences() {
        // "a a" on row 0 (cols 0 and 2) and "a" on row 1 → three matches total.
        let cells = [cell(0, 0, 'a'), cell(2, 0, 'a'), cell(0, 1, 'a')];
        assert_eq!(count_in_grid(&cells, "a", 10, 2), 3);
        // smart-case: lowercase term counts case-insensitively.
        let caps = [cell(0, 0, 'A'), cell(1, 0, 'b')];
        assert_eq!(count_in_grid(&caps, "ab", 10, 1), 1);
        // an uppercase term is case-sensitive (no match), and empty term is zero.
        assert_eq!(count_in_grid(&caps, "AB", 10, 1), 0);
        assert_eq!(count_in_grid(&caps, "", 10, 1), 0);
    }
}

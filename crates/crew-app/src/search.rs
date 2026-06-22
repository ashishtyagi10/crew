//! Scrollback search: `/find <term>` scrolls the focused terminal back to the
//! most recent line containing `term`.
use crate::app::CrewApp;
use crate::pane::PaneContent;
use crew_term::{RenderCell, TermModel};

/// Safety bound on how many lines a single search scrolls through.
const MAX_STEPS: usize = 5000;

/// Whether any row of the `cols × rows` grid (rebuilt from `cells`) contains `term`.
pub(crate) fn grid_contains(cells: &[RenderCell], term: &str, cols: u16, rows: u16) -> bool {
    if term.is_empty() {
        return false;
    }
    for r in 0..rows {
        let mut line = vec![' '; cols as usize];
        for cell in cells.iter().filter(|c| c.row == r) {
            if (cell.col as usize) < line.len() {
                line[cell.col as usize] = cell.c;
            }
        }
        if line.iter().collect::<String>().contains(term) {
            return true;
        }
    }
    false
}

impl CrewApp {
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
        // scroll never showed); report misses since the scroll alone can't.
        if searched {
            if found {
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
}

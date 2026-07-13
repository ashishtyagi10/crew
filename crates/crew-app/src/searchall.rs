//! Cross-pane scrollback search: `/findall <term>` counts smart-case matches
//! in every terminal pane's **full** scrollback (hidden panes included),
//! focuses the first matching pane, and runs the normal `/find` there so the
//! view scrolls to the match and repeat-`/find` continues upward. Counting
//! pages a whole screen per grid snapshot — the naive one-`cells()`-per-line
//! walk measured ~1s/pane on a fat history, far past what the winit thread
//! tolerates; paging is ~rows× fewer snapshots.
use crate::app::CrewApp;
use crate::pane::PaneContent;
use crate::search::{needle, rows_text};
use crew_term::{PtyTerm, TermModel};

/// Total scrollback lines examined per pane (matches `dump::capture_scrollback`).
const MAX_LINES: usize = 50_000;

/// Non-overlapping `needle` occurrences across `lines` (pre-folded).
fn count_lines(lines: &[String], needle: &str) -> usize {
    lines.iter().map(|l| l.matches(needle).count()).sum()
}

/// Smart-case match count of `term` across the pane's full scrollback,
/// paging by whole screens and restoring the viewport where it found it.
/// Uses the same ASCII fold as `/find`, so a `/findall` hit is always a
/// `/find` hit in the pane it lands on.
fn count_scrollback(pty: &mut PtyTerm, cols: u16, rows: u16, term: &str) -> usize {
    let (needle, ci) = needle(term);
    if needle.is_empty() {
        return 0;
    }
    let start = pty.display_offset();
    // Page up to the top of the scrollback.
    loop {
        let before = pty.display_offset();
        pty.scroll(rows as i32);
        if pty.display_offset() == before {
            break;
        }
    }
    // The top screen contributes every row; each page down then reveals
    // exactly `delta` new rows at the bottom (delta < rows on the last page
    // — slicing the tail avoids double-counting the overlap).
    let mut count = count_lines(&rows_text(&pty.cells(false), cols, rows, ci), &needle);
    let mut seen = rows as usize;
    while pty.display_offset() > 0 && seen < MAX_LINES {
        let before = pty.display_offset();
        pty.scroll(-(rows as i32));
        let delta = before - pty.display_offset();
        if delta == 0 {
            break;
        }
        let lines = rows_text(&pty.cells(false), cols, rows, ci);
        count += count_lines(&lines[rows as usize - delta..], &needle);
        seen += delta;
    }
    pty.scroll_to_bottom();
    if start > 0 {
        pty.scroll(start as i32);
    }
    count
}

impl CrewApp {
    /// `/findall <term>` — see the module doc.
    pub(crate) fn find_all(&mut self, term: &str) {
        let term = term.trim();
        if term.is_empty() {
            self.set_status("usage: /findall <text>".to_string());
            return;
        }
        let mut hits: Vec<(usize, usize)> = Vec::new();
        for i in 0..self.panes.len() {
            let (cols, rows) = (self.panes[i].grid.cols, self.panes[i].grid.rows);
            if let PaneContent::Terminal(t) = &mut self.panes[i].content {
                let n = count_scrollback(&mut t.pty, cols, rows, term);
                if n > 0 {
                    hits.push((i, n));
                }
            }
        }
        if hits.is_empty() {
            self.set_status(format!("no match for '{term}' in any pane"));
            return;
        }
        // Land on the first matching pane (reconcile restores it if hidden)
        // and run the per-pane find there FROM THE LIVE BOTTOM — /find only
        // searches upward, so a pane left scrolled up would strand at the
        // top missing a match that sits below its old view.
        self.focused = hits[0].0;
        self.input.focused = false;
        if let Some(pane) = self.panes.get_mut(self.focused) {
            if let PaneContent::Terminal(t) = &mut pane.content {
                t.pty.scroll_to_bottom();
            }
        }
        self.last_find = None;
        self.find_in_terminal(term);
        // Then overwrite the per-pane status with the fleet-wide summary:
        // total count, pane count, and the 1-based pane numbers (the same
        // numbers Cmd+1..9 and the tile badges use).
        let total: usize = hits.iter().map(|&(_, n)| n).sum();
        let where_list: Vec<String> = hits.iter().map(|&(i, _)| format!("#{}", i + 1)).collect();
        self.set_status(format!(
            "{total} match{} for '{term}' in {} pane{} ({})",
            if total == 1 { "" } else { "es" },
            hits.len(),
            if hits.len() == 1 { "" } else { "s" },
            where_list.join(" ")
        ));
    }
}

#[cfg(test)]
#[path = "searchall_tests.rs"]
mod tests;

//! Cross-pane scrollback search: `/findall <term>` counts smart-case matches
//! in every terminal pane's **full** scrollback (hidden panes included),
//! focuses the first matching pane, and runs the normal `/find` there so the
//! view scrolls to the match and repeat-`/find` continues upward. Bounded
//! work on the winit thread: `capture_scrollback` caps at `dump::MAX_LINES`
//! per pane and restores each viewport where it found it.
use crate::app::CrewApp;
use crate::pane::PaneContent;

/// One pane's result: `(pane index, match count)`.
type PaneHits = (usize, usize);

/// Smart-case occurrence count of `term` in `text` (case-insensitive unless
/// the term has an uppercase letter — the `/find` rule).
fn count_matches(text: &str, term: &str) -> usize {
    let ci = !term.chars().any(char::is_uppercase);
    if ci {
        text.to_lowercase().matches(&term.to_lowercase()).count()
    } else {
        text.matches(term).count()
    }
}

impl CrewApp {
    /// `/findall <term>` — see the module doc.
    pub(crate) fn find_all(&mut self, term: &str) {
        let term = term.trim();
        if term.is_empty() {
            self.set_status("usage: /findall <text>".to_string());
            return;
        }
        let mut hits: Vec<PaneHits> = Vec::new();
        for i in 0..self.panes.len() {
            let (cols, rows) = (self.panes[i].grid.cols, self.panes[i].grid.rows);
            if let PaneContent::Terminal(t) = &mut self.panes[i].content {
                let text = crate::dump::capture_scrollback(&mut t.pty, cols, rows);
                let n = count_matches(&text, term);
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
        // and run the per-pane find there: it scrolls to the most recent
        // match and seeds last_find, so a follow-up /find steps upward.
        self.focused = hits[0].0;
        self.input.focused = false;
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

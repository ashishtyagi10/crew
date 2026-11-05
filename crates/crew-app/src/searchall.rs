//! Cross-pane scrollback search: `/findall <term>` counts smart-case matches
//! in every terminal pane's **full** scrollback (hidden panes included),
//! focuses the first matching pane, and runs the normal `/find` there so the
//! view scrolls to the match and repeat-`/find` continues upward. Repeating
//! `/findall` with the same term cycles to the NEXT matching pane
//! (wrapping) — n/N stepping at fleet granularity. Counting reads each
//! pane's grid text in one pass (`find_back`): no screen of cells is built
//! and no view moves.
use crate::app::CrewApp;
use crate::findback::matches_in;
use crate::pane::PaneContent;
use crate::search::needle;
use crew_term::PtyTerm;

/// Smart-case match count of `term` across the pane's full scrollback, in
/// one pass over its grid (`find_back`) with the view left where it is.
/// Uses the same fold as `/find`, so a `/findall` hit is always a `/find`
/// hit in the pane it lands on.
fn count_scrollback(pty: &PtyTerm, term: &str) -> usize {
    let (needle, ci) = needle(term);
    let mut count = 0;
    pty.find_back(0, &mut |l| {
        count += matches_in(l, &needle, ci);
        false
    });
    count
}

/// Errors across a pane's whole scrollback, counted the way `/findall`
/// counts matches.
fn count_errors(pty: &PtyTerm) -> usize {
    let mut count = 0;
    pty.find_back(0, &mut |l| {
        count += usize::from(crate::errscan::looks_like_error(l));
        false
    });
    count
}

impl CrewApp {
    /// `/errorsall` — which panes have failures in them, and how many.
    ///
    /// The fleet-wide question `/errors` answers one pane at a time. With six
    /// agents running, "which of these went wrong" is the thing you want
    /// before you go looking in any of them.
    pub(crate) fn find_errors_everywhere(&mut self) {
        let mut hits: Vec<(usize, usize)> = Vec::new();
        for i in 0..self.panes.len() {
            if let PaneContent::Terminal(t) = &self.panes[i].content {
                let n = count_errors(&t.pty);
                if n > 0 {
                    hits.push((i, n));
                }
            }
        }
        let Some(&(first, _)) = hits.first() else {
            self.set_status("no errors in any pane");
            return;
        };
        // Land on the first pane with errors and walk it to its most recent
        // one, so the command ends somewhere useful rather than on a tally.
        self.focused = first;
        self.input.focused = false;
        self.reconcile_grid();
        self.last_find = None;
        self.find_error_in_terminal();
        let total: usize = hits.iter().map(|&(_, n)| n).sum();
        let where_list: Vec<String> = hits
            .iter()
            .map(|&(i, n)| {
                let mark = if i == self.focused { "\u{2192}" } else { "" };
                format!("{mark}#{} ({n})", i + 1)
            })
            .collect();
        let panes = hits.len();
        let plural = if total == 1 { "" } else { "s" };
        self.set_status(format!(
            "{total} error{plural} in {panes} pane{}: {}",
            if panes == 1 { "" } else { "s" },
            where_list.join(" ")
        ));
    }

    /// `/findall <term>` — see the module doc.
    pub(crate) fn find_all(&mut self, term: &str) {
        let term = term.trim();
        if term.is_empty() {
            self.set_status("usage: /find all <text>".to_string());
            return;
        }
        let mut hits: Vec<(usize, usize)> = Vec::new();
        for i in 0..self.panes.len() {
            if let PaneContent::Terminal(t) = &self.panes[i].content {
                let n = count_scrollback(&t.pty, term);
                if n > 0 {
                    hits.push((i, n));
                }
            }
        }
        if hits.is_empty() {
            self.last_findall = None;
            self.set_status(format!("no match for \u{201c}{term}\u{201d} in any pane"));
            return;
        }
        // Repeating the same term cycles: land on the matching pane AFTER
        // the focused one (wrapping); a fresh term starts at the first.
        let repeat = self.last_findall.as_deref() == Some(term);
        self.last_findall = Some(term.to_string());
        let target = match hits.iter().position(|&(i, _)| i == self.focused) {
            Some(pos) if repeat => hits[(pos + 1) % hits.len()].0,
            _ => hits[0].0,
        };
        // Land on the target pane (reconcile restores it if hidden) and run
        // the per-pane find there FROM THE LIVE BOTTOM — /find only
        // searches upward, so a pane left scrolled up would strand at the
        // top missing a match that sits below its old view.
        self.focused = target;
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
        // The landed pane is arrow-marked so cycling shows where you are.
        let where_list: Vec<String> = hits
            .iter()
            .map(|&(i, _)| {
                let mark = if i == self.focused { "\u{2192}" } else { "" };
                format!("{mark}#{}", i + 1)
            })
            .collect();
        self.set_status(format!(
            "{total} match{} for \u{201c}{term}\u{201d} in {} pane{} ({})",
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

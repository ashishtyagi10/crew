//! How far a turn's change reaches, for the head of its receipt.
//!
//! The head said WHAT changed (`2 files changed (+12 −3)`) but never who
//! depends on it, so a one-line edit to a helper forty files call read the
//! same as one to a private function nothing calls. `/review` already
//! greps for the callers of what a diff changed (`reviewcallers`); the
//! receipt now asks the same question and folds the answer into its head
//! (` · reaches 6 callers in 3 files`) — a number, not a new row.
use std::collections::BTreeSet;
use std::path::Path;

use super::reviewcallers::{call_sites, changed_symbols, is_test_path};

/// `(call sites, files)` outside tests that use a name `patch` defines or
/// edits inside, in `dir`'s tree as it is now; `None` when nothing does.
/// Tests are left out: they call everything, and a count they inflate says
/// less about who the change can break.
pub(crate) fn reach(dir: &Path, patch: &str) -> Option<(usize, usize)> {
    let mut sites = BTreeSet::new();
    let mut files = BTreeSet::new();
    for name in changed_symbols(patch) {
        for (path, line, _) in call_sites(dir, &name) {
            if is_test_path(&path) {
                continue;
            }
            sites.insert((path.clone(), line));
            files.insert(path);
        }
    }
    (!sites.is_empty()).then_some((sites.len(), files.len()))
}

/// `summary` (`changed::summary`'s line) with the reach after its file list
/// and before its once-a-session hint, which the pane reads off the end.
pub(crate) fn reached(summary: &str, reach: Option<(usize, usize)>) -> String {
    let Some((n, f)) = reach else {
        return summary.to_string();
    };
    let s = |k: usize| if k == 1 { "" } else { "s" };
    let note = format!(" \u{b7} reaches {n} caller{} in {f} file{}", s(n), s(f));
    match summary.split_once(" \u{2014} ") {
        Some((list, hint)) => format!("{list}{note} \u{2014} {hint}"),
        None => format!("{summary}{note}"),
    }
}

#[cfg(test)]
#[path = "receiptreach_tests.rs"]
mod tests;

//! A review leaves the tree as it found it, even when crew cannot say how.
//!
//! `handsoff` runs each CLI crew knows in its read-only mode. A manifest
//! agent's command is one crew does not know, and its review used to run with
//! its hands on: elected as the reviewer, a CLI that edits in place could
//! "fix" what it was asked to judge (the loop 39 S8 failure, opencode
//! rewriting two callers), and nothing said so. For such an agent the tree is
//! hashed before the call; any change after is put back and named in one
//! smith row, so the person who asked still decides what to fix.
//!
//! Only unknown CLIs are guarded: putting the tree back would also undo a
//! file the user saved in their editor during the review, a price worth
//! paying only where nothing else keeps the reviewer's hands off.
use std::path::Path;
use std::time::Duration;

use super::adapter::Adapter;

/// How many edited paths the row names before counting the rest.
const NAMED: usize = 4;

/// `a`'s read-only call of `body`, plus the `!` row to show when `a` had no
/// read-only mode and edited `dir` anyway (the edits already put back).
pub(crate) fn call(
    dir: &Path,
    a: &dyn Adapter,
    body: &str,
    timeout: Duration,
) -> (Result<String, String>, Option<String>) {
    if a.hands_off() {
        return (a.call_read_only(body, timeout), None);
    }
    // Not a git repository: there is no diff to review either, so the call
    // below is never reached without a base in practice.
    let base = super::checkpoint::worktree_tree(dir).ok();
    let reply = a.call_read_only(body, timeout);
    let note = base.and_then(|b| put_back(dir, &b));
    (reply, note)
}

/// Restore `dir` to the tree `base` if it moved, and say what happened.
fn put_back(dir: &Path, base: &str) -> Option<String> {
    let changes = super::changed::since(dir, base).ok()?;
    if changes.is_empty() {
        return None;
    }
    let paths: Vec<String> = changes.into_iter().map(|(_, p)| p).collect();
    let named = name(&paths);
    Some(match super::checkpoint::restore(dir, base) {
        Ok(_) => format!("! the reviewer edited {named} — restored"),
        Err(e) => format!("! the reviewer edited {named} and it could not be put back: {e}"),
    })
}

/// `a.rs, b.rs, +3 more`.
fn name(paths: &[String]) -> String {
    let mut s = paths[..paths.len().min(NAMED)].join(", ");
    if paths.len() > NAMED {
        s.push_str(&format!(", +{} more", paths.len() - NAMED));
    }
    s
}

// The fake reviewer is a shell script.
#[cfg(all(test, unix))]
#[path = "reviewguard_tests.rs"]
mod tests;

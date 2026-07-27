//! `/diff` — codex-style working-tree diff. Read-only and bounded, so it runs
//! inline as a quick construct (see `commands::handle`).
//!
//! It asks the same question the end-of-task note answers ("what is different
//! now?") and must therefore answer it the same way, or the note points at a
//! command that contradicts it. It did not: `git diff --stat` compares the
//! working tree to the INDEX, so a file the agent created was untracked and
//! invisible, and a file anything had staged was excluded too — the two most
//! likely states after an agent edits a repository, both reported as "working
//! tree clean".
//!
//! So the comparison is built the way the checkpoint builds one: everything
//! that exists, `.gitignore` respected, through a throwaway index that leaves
//! HEAD, the user's index and every branch untouched.
use std::path::Path;

use crate::PluginEvent;

use super::checkpoint::{git, worktree_tree};
use super::relay::msg;

/// The empty tree, asked of git rather than hard-coded — the well-known
/// `4b825dc…` is the SHA-1 value and is wrong in a SHA-256 repository.
fn empty_tree(dir: &Path) -> Result<String, String> {
    git(dir, &["hash-object", "-t", "tree", "/dev/null"], None)
}

/// `--stat` for everything in `dir` that differs from the last commit,
/// untracked files included. On an unborn branch there is no HEAD to compare
/// against, so the first files in a fresh repository still show as additions
/// rather than as an error.
fn worktree_stat(dir: &Path) -> Result<String, String> {
    let tree = worktree_tree(dir)?;
    let base = match git(dir, &["rev-parse", "--verify", "HEAD^{tree}"], None) {
        Ok(head) => head,
        Err(_) => empty_tree(dir)?,
    };
    git(
        dir,
        &[
            "diff-tree",
            "-r",
            "--stat",
            &base,
            &tree,
            // Crew's own transcript is not the user's work, and `.crew/` is
            // only gitignored in crew's own repo. Same exclusion as the
            // end-of-task note, so the two views cannot disagree.
            "--",
            ":!.crew",
        ],
        None,
    )
}

/// Format `git diff --stat` output for the crew pane. Empty (clean tree) →
/// a friendly line; long output is bounded so a huge repo can't flood the pane.
pub(crate) fn diff_report(raw_stat: &str) -> String {
    let trimmed = raw_stat.trim();
    if trimmed.is_empty() {
        return "working tree clean \u{2014} no changes".to_string();
    }
    const CAP: usize = 4000;
    if trimmed.len() > CAP {
        let mut s: String = trimmed.chars().take(CAP).collect();
        s.push_str("\n\u{2026} (diff truncated)");
        s
    } else {
        trimmed.to_string()
    }
}

/// `/diff` — show everything that differs from the last commit, bounded.
pub(crate) fn diff_cmd(
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let dir = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => return emit(msg("agent smith", format!("diff failed: {e}"))),
    };
    match worktree_stat(&dir) {
        Ok(raw) => emit(msg("agent smith", diff_report(&raw))),
        Err(e) => emit(msg("agent smith", format!("diff failed: {e}"))),
    }
}

#[cfg(test)]
#[path = "diff_tests.rs"]
mod tests;

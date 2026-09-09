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

/// How much of the patch `/diff` shows inline, in chars — wider than the
/// end-of-task note's [`super::taskdiff::PATCH_CAP`] because here the reader
/// ASKED for the whole thing; bounded all the same, since a pane is not a
/// pager and a repo-wide reformat would bury the transcript.
pub(crate) const PATCH_CAP: usize = 30_000;

/// `--stat` for everything in `dir` that differs from the last commit,
/// untracked files included. On an unborn branch there is no HEAD to compare
/// against, so the first files in a fresh repository still show as additions
/// rather than as an error.
fn worktree_stat(dir: &Path) -> Result<String, String> {
    let tree = worktree_tree(dir)?;
    let base = super::changed::head_tree(dir)?;
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
            super::changed::NOT_CREW,
        ],
        None,
    )
}

/// The same comparison as [`worktree_stat`], as a patch.
fn worktree_patch(dir: &Path) -> Result<String, String> {
    super::changed::patch(dir, &super::changed::head_tree(dir)?)
}

/// The `/diff` reply: the stat block, then the patch in the ```diff fence the
/// pane renders (clipped on a line boundary at [`PATCH_CAP`]). An empty stat
/// (clean tree) is a friendly line instead; a long stat is bounded so a huge
/// repo cannot flood the pane before the patch even starts.
pub(crate) fn diff_report(raw_stat: &str, patch: &str) -> String {
    let trimmed = raw_stat.trim();
    if trimmed.is_empty() {
        return "working tree clean \u{2014} no changes".to_string();
    }
    const CAP: usize = 4000;
    let mut out = if trimmed.len() > CAP {
        let mut s: String = trimmed.chars().take(CAP).collect();
        s.push_str("\n\u{2026} (diff truncated)");
        s
    } else {
        trimmed.to_string()
    };
    if !patch.trim().is_empty() {
        out.push_str("\n\n");
        out.push_str(&super::taskdiff::fenced(&super::taskdiff::clip(
            patch, PATCH_CAP, "",
        )));
    }
    out
}

/// `/diff` — show everything that differs from the last commit, bounded.
pub(crate) fn diff_cmd(
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let dir = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => return emit(msg("agent smith", format!("diff failed: {e}"))),
    };
    match worktree_stat(&dir).and_then(|stat| Ok((stat, worktree_patch(&dir)?))) {
        Ok((stat, patch)) => emit(msg("agent smith", diff_report(&stat, &patch))),
        Err(e) => emit(msg("agent smith", format!("diff failed: {e}"))),
    }
}

#[cfg(test)]
#[path = "diff_tests.rs"]
mod tests;

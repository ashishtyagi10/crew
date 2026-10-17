//! What changed, in front of a task about something that broke.
//!
//! Measured live (2026-10-16, DashScope, a scratch clone): with a swapped
//! colour channel planted in an uncommitted edit, "cargo test -p crew-theme
//! started failing after my last change. Find the root cause and fix it"
//! ran `git log` three times, never `git diff`, spent its four calls and
//! called itself done with the bug still there. The routing line had even
//! said `tree dirty (1 file)`.
//!
//! A senior engineer's first question about a regression is what changed
//! since it worked. So a task that talks about breakage carries the answer
//! up front: the uncommitted diff (stat, then whole files of hunks up to
//! [`CAP`]), or, in a clean tree, the last commit's. Any other task is sent
//! exactly as before; the diff is evidence for one kind of question, not
//! furniture for every prompt.
use std::path::{Path, PathBuf};

use super::checkpoint::git;

/// Chars of hunks carried. Whole files are kept until the next would pass
/// it, so a hunk is never cut in the middle; past it, the rest are named.
pub(crate) const CAP: usize = 4_000;

/// Words that make a task about something that broke.
const BREAKAGE: &[&str] = &[
    "fail",
    "broke",
    "break",
    "regress",
    "no longer",
    "stopped working",
    "doesn't work",
    "does not work",
    "not working",
    "since my",
    "after my",
    "last change",
    "panic",
    "crash",
];

/// The block for `task`, from the broker's project directory; `None` when
/// the task is not about breakage or there is nothing to show.
pub(crate) fn block(task: &str) -> Option<String> {
    if !about_breakage(task) {
        return None;
    }
    let dir = std::env::var("CREW_PROJECT_DIR")
        .map(PathBuf::from)
        .or_else(|_| std::env::current_dir())
        .ok()?;
    block_at(&dir)
}

pub(crate) fn about_breakage(task: &str) -> bool {
    let t = task.to_lowercase();
    BREAKAGE.iter().any(|w| t.contains(w))
}

/// The block for the repository at `dir`, or `None` outside one.
pub(crate) fn block_at(dir: &Path) -> Option<String> {
    let stat = git(dir, &["diff", "HEAD", "--stat"], None).ok()?;
    let (what, stat, patch) = match stat.is_empty() {
        false => (
            "uncommitted",
            stat,
            git(dir, &["diff", "HEAD", "-U2"], None).ok()?,
        ),
        true => (
            "the last commit; the tree is clean",
            git(dir, &["show", "--stat", "--format=%h %s", "HEAD"], None).ok()?,
            git(dir, &["show", "-U2", "--format=", "HEAD"], None).ok()?,
        ),
    };
    Some(format!(
        "WHAT CHANGED ({what}). When something that worked has broken, this \
         is the first suspect: read it before you search.\n{stat}\n\n{}",
        whole_files(&patch, CAP)
    ))
}

/// `patch`'s files, whole, while they fit in `cap`; the rest are named.
/// One file larger than `cap` on its own is cut at a line, and says so.
pub(crate) fn whole_files(patch: &str, cap: usize) -> String {
    let files: Vec<&str> = patch.split("\ndiff --git ").collect();
    let mut out = String::new();
    let mut left: Vec<String> = Vec::new();
    for (i, f) in files.iter().enumerate() {
        let f = if i == 0 {
            f.to_string()
        } else {
            format!("diff --git {f}")
        };
        if left.is_empty() && out.len() + f.len() + 1 <= cap {
            out.push_str(&f);
            out.push('\n');
        } else if out.is_empty() {
            let mut cut = cap;
            while !f.is_char_boundary(cut) {
                cut -= 1;
            }
            let at = f[..cut].rfind('\n').unwrap_or(cut);
            out = format!("{}\n… (cut: this file's diff is longer)\n", &f[..at]);
        } else {
            left.push(file_name(&f));
        }
    }
    if !left.is_empty() {
        out.push_str(&format!("… not shown: {}\n", left.join(", ")));
    }
    out
}

/// `a/x.rs b/x.rs` from a file's `diff --git` line, as `x.rs`.
fn file_name(f: &str) -> String {
    let head = f.lines().next().unwrap_or_default();
    head.rsplit(" b/").next().unwrap_or(head).to_string()
}

#[cfg(test)]
#[path = "whatchanged_tests.rs"]
mod tests;

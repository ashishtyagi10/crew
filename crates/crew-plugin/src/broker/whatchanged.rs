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

/// Word stems that make a task about something that broke. A word counts
/// when it is a stem plus an ending in [`ENDINGS`] — `broke`, `broken`,
/// `failing`, `crashes` — and not when the stem merely opens it. Matched as
/// plain substrings they were also in `broker` (this repository's own
/// directory), `breakpoint` and `failover`, and a greeting that recalled a
/// path under `broker/` was sent the last commit's diff (live, 2026-11-14).
const STEMS: &[&str] = &["fail", "broke", "break", "regress", "panic", "crash"];

/// What may follow a stem and leave it the same word.
const ENDINGS: &[&str] = &[
    "", "s", "es", "ed", "ing", "n", "ure", "ures", "ion", "ions", "age", "ked", "king",
];

/// Phrases that say it in more than one word; these are matched as written.
const PHRASES: &[&str] = &[
    "no longer",
    "stopped working",
    "doesn't work",
    "does not work",
    "not working",
    "since my",
    "after my",
    "last change",
];

/// The block for `asked`, from the broker's project directory; `None` when
/// it is not about breakage or there is nothing to show. `asked` is the
/// user's own request, never the framed prompt: the skill catalog and the
/// recalled turns in front of it are full of other people's words.
pub(crate) fn block(asked: &str) -> Option<String> {
    if !about_breakage(asked) {
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
    PHRASES.iter().any(|p| t.contains(p))
        || t.split(|c: char| !c.is_alphanumeric()).any(|w| {
            STEMS
                .iter()
                .any(|s| w.strip_prefix(s).is_some_and(|end| ENDINGS.contains(&end)))
        })
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

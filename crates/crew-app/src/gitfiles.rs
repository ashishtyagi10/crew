//! Which files in a repository git calls changed — for the Explorer, as VS Code marks its tree.
//!
//! The nav and the cards already ask `git status --porcelain --branch` of every pane's folder
//! every few seconds ([`crate::gitfleet`]), and kept only the count of the lines under the
//! header. Those lines ARE the per-file answer; this keeps them, so the tree's marks cost no
//! extra git. Porcelain paths are relative to the repository's root, not to the folder asked
//! about, so the root is found by walking up to its `.git` (a folder, or a worktree's file).
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::git::GitInfo;

/// What git says of one path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mark {
    Modified,
    Added,
    Untracked,
    Deleted,
    Conflict,
}

impl Mark {
    /// The letter VS Code's tree writes beside the name.
    pub(crate) fn letter(self) -> &'static str {
        match self {
            Mark::Modified => "M",
            Mark::Added => "A",
            Mark::Untracked => "U",
            Mark::Deleted => "D",
            Mark::Conflict => "!",
        }
    }
}

/// Every changed path of one repository, absolute.
#[derive(Debug, Default)]
pub(crate) struct Files {
    marks: HashMap<PathBuf, Mark>,
}

impl Files {
    /// The mark on `path`: its own, or an untracked folder's it sits in (git lists a new
    /// folder once, not each file inside it).
    pub(crate) fn mark(&self, path: &Path) -> Option<Mark> {
        self.marks.get(path).copied().or_else(|| {
            path.ancestors().skip(1).find_map(|a| {
                self.marks
                    .get(a)
                    .filter(|m| **m == Mark::Untracked)
                    .copied()
            })
        })
    }

    /// Whether anything under the folder `dir` is changed.
    pub(crate) fn holds_changes(&self, dir: &Path) -> bool {
        self.marks.keys().any(|p| p.starts_with(dir) && p != dir)
    }
}

/// [`crate::git::query`], with the per-file marks it used to drop.
pub(crate) fn query(dir: &Path) -> Option<(GitInfo, Files)> {
    let out = crate::git::run(dir, &["status", "--porcelain", "--branch"])?;
    let info = crate::git::parse_status(&out)?;
    let files = repo_root(dir).map_or_else(Files::default, |root| parse(&out, &root));
    Some((info, files))
}

/// The folder holding `.git`, from `dir` up.
fn repo_root(dir: &Path) -> Option<PathBuf> {
    dir.ancestors()
        .find(|a| a.join(".git").exists())
        .map(Path::to_path_buf)
}

/// The marks in porcelain v1 output, made absolute under `root`.
pub(crate) fn parse(out: &str, root: &Path) -> Files {
    let marks = out
        .lines()
        .filter(|l| !l.starts_with("## ") && l.len() > 3)
        .filter_map(|l| {
            let (xy, path) = l.split_at(3);
            let (x, y) = (xy.as_bytes()[0], xy.as_bytes()[1]);
            let mark = match (x, y) {
                (b'?', b'?') => Mark::Untracked,
                (b'U', _) | (_, b'U') | (b'A', b'A') | (b'D', b'D') => Mark::Conflict,
                (b'A', _) => Mark::Added,
                (b'D', _) | (_, b'D') => Mark::Deleted,
                (b'!', _) => return None,
                _ => Mark::Modified,
            };
            // A rename lists `old -> new`: the tree shows the new name.
            let path = path.rsplit(" -> ").next().unwrap_or(path);
            let path = path.trim_matches('"').trim_end_matches('/');
            Some((root.join(path), mark))
        })
        .collect();
    Files { marks }
}

#[cfg(test)]
#[path = "gitfiles_tests.rs"]
mod tests;

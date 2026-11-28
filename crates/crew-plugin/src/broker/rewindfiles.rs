//! What a rewind would overwrite, and whose it is.
//!
//! `checkpoint::restore` reverts or deletes EVERY non-ignored file that
//! differs from the task's starting tree, the user's own edits included. So
//! the pane must be told which files, which of them crew did not touch, and —
//! when that cannot be known — that it cannot be known. Uncertainty never
//! reads as "no outside edits".
use std::collections::BTreeSet;
use std::path::Path;

use super::rewind::Log;

/// Paths named in the confirmation; the rest are counted.
pub(crate) const SHOWN: usize = 5;

/// Everything a rewind to one task would change.
#[derive(Debug, Default)]
pub(crate) struct Report {
    /// Every path that goes back, the ones edited outside crew first.
    pub files: Vec<String>,
    /// How many of the leading `files` were edited outside crew.
    pub yours: usize,
    /// Edits made outside crew cannot be ruled out: a tree is unknown or git
    /// failed. The answer to "were there any?" is then "maybe".
    pub unsure: bool,
    /// Fingerprint of `files` and their contents (see [`stamp`]).
    pub stamp: String,
}

/// What a rewind to `task` costs, in `dir`.
pub(crate) fn report(log: &Log, dir: &Path, task: u64) -> Result<Report, String> {
    let Some(point) = log.get(task) else {
        return Err(format!(
            "task #{task} is not remembered \u{2014} the broker restarted, or it is more than {} tasks back",
            super::rewind::KEEP
        ));
    };
    let Some(pre) = &point.pre else {
        return Ok(Report::default());
    };
    let tree = format!("{pre}^{{tree}}");
    super::checkpoint::git(dir, &["cat-file", "-e", &tree], None)
        .map_err(|_| "its snapshot is gone \u{2014} git collected it".to_string())?;
    let now = super::checkpoint::worktree_tree(dir)?;
    let mut files: Vec<String> = super::changed::between(dir, pre, &now)?
        .into_iter()
        .map(|(_, p)| p)
        .collect();
    let (outside, unsure) = outside(log, dir, task, &now);
    files.sort_by_key(|p| !outside.contains(p));
    Ok(Report {
        yours: files.iter().take_while(|p| outside.contains(*p)).count(),
        stamp: stamp(dir, pre, &now)?,
        files,
        unsure,
    })
}

/// Paths touched by hand after `task` began, and whether that is incomplete.
///
/// The gaps are the stretches no task owns: between one task's end and the
/// next one's start, and from the last end to now. The target's own gap before
/// its start is inside the tree it began on, so the restore keeps it — it adds
/// no path, though its being unknown still makes the answer a "maybe" (a first
/// task, a concurrent start and a failed finish all look the same from here).
fn outside(log: &Log, dir: &Path, task: u64, now: &str) -> (BTreeSet<String>, bool) {
    let mut paths = BTreeSet::new();
    let mut unsure = false;
    let mut gap = |a: &str, b: &str| match super::changed::between(dir, a, b) {
        Ok(c) => paths.extend(c.into_iter().map(|(_, p)| p)),
        Err(_) => unsure = true,
    };
    let mut unknown = false;
    for p in log.points.iter().filter(|p| p.task >= task) {
        match (&p.left, &p.pre) {
            (Some(left), Some(pre)) if p.task > task => gap(left, pre),
            (Some(_), Some(_)) => {}
            _ => unknown = true,
        }
    }
    match log.points.last().and_then(|p| p.post.as_deref()) {
        Some(end) => gap(end, now),
        None => unknown = true,
    }
    (paths, unsure || unknown)
}

/// A digest of what differs from `base` right now, contents included: a file
/// swapped for another, or edited again, changes it. `.crew/` is left out as
/// everywhere (`changed`) — the live transcript churns and would refuse every
/// confirm. Empty when nothing differs.
fn stamp(dir: &Path, base: &str, now: &str) -> Result<String, String> {
    use std::hash::{Hash, Hasher};
    if base == now {
        return Ok(String::new());
    }
    let raw = super::checkpoint::git(dir, &["diff-tree", "-r", base, now], None)?;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for line in raw.lines() {
        let path = line.split_once('\t').map_or("", |(_, p)| p);
        if !super::changed::is_crew_artifact(path) {
            line.hash(&mut h);
        }
    }
    Ok(format!("{:016x}", h.finish()))
}

/// Where the checkpoint labelled `label` sits in the list `undo #N` counts,
/// newest first (so the one just taken is 1); 0 if it is not there.
pub(crate) fn pin_number(dir: &Path, label: &str) -> u32 {
    super::undo::numbered(dir)
        .iter()
        .position(|(_, l)| l == label)
        .map_or(0, |i| i as u32 + 1)
}

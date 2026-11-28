//! Rewind: a turn went wrong three messages ago, and "undo that" only reaches
//! the newest checkpoint while the conversation keeps standing on the bad
//! history. This takes BOTH back to the moment one earlier task began — the
//! working tree to what it was, the thread to what had been said before it.
//!
//! The mapping the checkpoints never had: `checkpoint::auto_snapshot` writes
//! no ref when the tree is unchanged, so "the ref of task N" does not always
//! exist. What every task does have is the TREE it began on (the answer
//! `last_tree` holds right after the snapshot call), and a tree hash is as good
//! as a ref for `restore` and `changed::since`, and it is pinned by whichever
//! ref first wrote it. So a [`Point`] records tree + start time per task.
//!
//! Two steps, because putting files back deletes work: the pane asks
//! (`confirm` false) and is told which files would change and whether any
//! of that is not crew's doing; only a second message, after the user said
//! yes to THAT list (`seen`), changes anything — and it pins the tree as it
//! stands first, so `undo #N` undoes the rewind itself.
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::PluginEvent;

use super::rewindfiles as files;
use super::session::Session;

/// Tasks remembered. Older ones answer "too far back" — a bound, like the
/// checkpoint refs' own (`checkpoint::KEEP`).
pub(super) const KEEP: usize = 40;

/// What one task began on.
#[derive(Debug, Clone, Default)]
pub(crate) struct Point {
    pub task: u64,
    /// When it began, unix ms: the thread keeps turns answered before it.
    pub at_ms: u64,
    /// The working tree it began on; `None` outside a git repository.
    pub pre: Option<String>,
    /// The tree it left, once it ended.
    pub post: Option<String>,
    /// The tree the PREVIOUS task left, when that one had ended before this
    /// began — the other half of "did anyone edit between the two?".
    pub left: Option<String>,
}

/// Every task's [`Point`], oldest first.
#[derive(Debug, Default)]
pub(crate) struct Log {
    pub(super) points: Vec<Point>,
}

pub(crate) type SharedLog = Arc<Mutex<Log>>;

pub(crate) fn lock(log: &SharedLog) -> std::sync::MutexGuard<'_, Log> {
    log.lock().unwrap_or_else(|e| e.into_inner())
}

impl Log {
    /// A task was admitted at `at_ms`.
    pub(crate) fn begin(&mut self, task: u64, at_ms: u64) {
        self.points.push(Point {
            task,
            at_ms,
            ..Default::default()
        });
        let over = self.points.len().saturating_sub(KEEP);
        self.points.drain(..over);
    }

    /// The tree `task` began on is `tree`. `left` is taken only when every
    /// earlier task has ended: with one still running, "what the last one
    /// left" is not yet a fact and the comparison would cry wolf.
    pub(crate) fn began_on(&mut self, task: u64, tree: &str) {
        let settled = self
            .points
            .iter()
            .filter(|p| p.task != task)
            .all(|p| p.pre.is_none() || p.post.is_some());
        let left = settled
            .then(|| self.points.iter().rev().find_map(|p| p.post.clone()))
            .flatten();
        if let Some(p) = self.points.iter_mut().find(|p| p.task == task) {
            p.pre = Some(tree.to_string());
            p.left = left;
        }
    }

    /// `task` ended leaving `tree`.
    pub(crate) fn ended_on(&mut self, task: u64, tree: String) {
        if let Some(p) = self.points.iter_mut().find(|p| p.task == task) {
            p.post = Some(tree);
        }
    }

    pub(super) fn get(&self, task: u64) -> Option<&Point> {
        self.points.iter().find(|p| p.task == task)
    }

    /// `task` and everything after it are gone with the rewind.
    fn drop_from(&mut self, task: u64) {
        self.points.retain(|p| p.task < task);
    }
}

/// Record the tree a task ended on (best-effort: outside git, nothing).
pub(crate) fn finish(session: &Session, dir: &Path, task: u64) {
    if let Ok(tree) = super::checkpoint::worktree_tree(dir) {
        lock(&session.rewind).ended_on(task, tree);
    }
}

/// The broker's answer to [`crate::PluginCommand::Rewind`]. `running` is how
/// many tasks are in flight: a rewind under a running task would be undone by
/// it the moment it wrote its next file.
pub(crate) fn handle(
    session: &Session,
    dir: &Path,
    running: usize,
    (task, confirm, seen): (u64, bool, &str),
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let fail = |error: String| emit_one(task, None, 0, error);
    if running > 0 {
        return emit(fail("stop it first (/stop)".into()));
    }
    let (rep, point) = {
        let log = lock(&session.rewind);
        match files::report(&log, dir, task) {
            Ok(rep) => (rep, log.get(task).cloned().unwrap_or_default()),
            Err(e) => return emit(fail(e)),
        }
    };
    if !confirm {
        return emit(emit_one(task, Some((&rep, false)), 0, String::new()));
    }
    // Yes was said to one list. A task, an autosave or the user's own hand
    // since then makes it a different one — and this deletes work.
    if rep.stamp != seen {
        return emit(fail(
            "files changed since you looked \u{2014} Ctrl+B again".into(),
        ));
    }
    let mut pin = 0;
    if let Some(pre) = point.pre.as_ref().filter(|_| !rep.files.is_empty()) {
        // Pinned first, so the rewind is itself one "undo that" away.
        let label = format!("before rewind to #{task}");
        if let Err(e) = super::checkpoint::snapshot(dir, &label) {
            return emit(fail(format!("could not pin the tree first: {e}")));
        }
        pin = files::pin_number(dir, &label);
        if let Err(e) = super::checkpoint::restore(dir, pre) {
            return emit(fail(format!("rewind failed: {e}")));
        }
        // The next task must write its own ref: the tree it begins on is no
        // longer the one `last_tree` remembers.
        *session.last_tree.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }
    super::thread::lock(&session.thread).keep_before(point.at_ms);
    // A resumed Claude Code session would carry the turns just dropped.
    super::clauderesume::Resume::shared().forget();
    super::steer::clear();
    *session.plan.lock().unwrap_or_else(|e| e.into_inner()) = None;
    super::gitpr::drop_pr(session);
    *super::undo::lock(&session.undo) = None;
    lock(&session.rewind).drop_from(task);
    emit(emit_one(task, Some((&rep, true)), pin, String::new()))
}

/// The event for `rep` (`applied` with it), or for a refusal when `rep` is
/// `None`. The names lead with the files edited outside crew, so a long list
/// never hides them behind "+N more".
fn emit_one(
    task: u64,
    rep: Option<(&files::Report, bool)>,
    pin: u32,
    error: String,
) -> PluginEvent {
    let Some((rep, applied)) = rep else {
        return PluginEvent::Rewind {
            task,
            files: 0,
            names: Vec::new(),
            yours: Vec::new(),
            outside: false,
            unsure: false,
            stamp: String::new(),
            pin,
            applied: false,
            error,
        };
    };
    let names: Vec<String> = rep.files.iter().take(files::SHOWN).cloned().collect();
    PluginEvent::Rewind {
        task,
        files: rep.files.len() as u32,
        yours: names.iter().take(rep.yours).cloned().collect(),
        names,
        outside: rep.yours > 0,
        unsure: rep.unsure,
        stamp: rep.stamp.clone(),
        pin,
        applied,
        error,
    }
}

#[cfg(test)]
#[path = "rewind_tests.rs"]
mod rewind_tests;
#[cfg(test)]
#[path = "rewindfiles_tests.rs"]
mod rewindfiles_tests;

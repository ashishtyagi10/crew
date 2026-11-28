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
//! (`confirm` false) and is told how many files would change and whether any
//! of that is not crew's doing; only a second message, after the user said
//! yes, changes anything — and it pins the tree as it stands first, so "undo
//! that" undoes the rewind itself.
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::PluginEvent;

use super::session::Session;

/// Tasks remembered. Older ones answer "too far back" — a bound, like the
/// checkpoint refs' own (`checkpoint::KEEP`).
const KEEP: usize = 40;

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
    points: Vec<Point>,
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

    fn get(&self, task: u64) -> Option<&Point> {
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

/// What going back to a task would change.
pub(crate) struct Report {
    pub files: usize,
    /// Some of those files were changed by someone other than crew's tasks.
    pub outside: bool,
}

/// Count what a rewind to `task` costs, in `dir`.
pub(crate) fn report(log: &Log, dir: &Path, task: u64) -> Result<Report, String> {
    let Some(point) = log.get(task) else {
        return Err(format!(
            "task #{task} is not remembered \u{2014} the broker restarted, or it is more than {KEEP} tasks back"
        ));
    };
    let Some(pre) = &point.pre else {
        return Ok(Report {
            files: 0,
            outside: false,
        });
    };
    let tree = format!("{pre}^{{tree}}");
    super::checkpoint::git(dir, &["cat-file", "-e", &tree], None)
        .map_err(|_| "its snapshot is gone \u{2014} git collected it".to_string())?;
    let files = super::changed::since(dir, pre)?.len();
    Ok(Report {
        files,
        outside: edited_outside(log, dir, task),
    })
}

/// Whether anything between `task` and now was edited by hand: a gap between
/// one task's end and the next one's start, or between the last end and now.
fn edited_outside(log: &Log, dir: &Path, task: u64) -> bool {
    let differs =
        |a: &str, b: &str| super::changed::between(dir, a, b).is_ok_and(|c| !c.is_empty());
    let later = || log.points.iter().filter(|p| p.task >= task);
    let gap = later().any(|p| match (&p.left, &p.pre) {
        (Some(left), Some(pre)) => differs(left, pre),
        _ => false,
    });
    let last_end = log.points.iter().rev().find_map(|p| p.post.clone());
    let now = last_end.is_some_and(|end| {
        super::checkpoint::worktree_tree(dir).is_ok_and(|now| differs(&end, &now))
    });
    gap || now
}

/// The broker's answer to [`crate::PluginCommand::Rewind`]. `running` is how
/// many tasks are in flight: a rewind under a running task would be undone by
/// it the moment it wrote its next file.
pub(crate) fn handle(
    session: &Session,
    dir: &Path,
    running: usize,
    (task, confirm): (u64, bool),
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let mut reply = |files: usize, outside: bool, applied: bool, error: String| {
        emit(PluginEvent::Rewind {
            task,
            files: files as u32,
            outside,
            applied,
            error,
        })
    };
    if running > 0 {
        return reply(0, false, false, "stop it first (/stop)".into());
    }
    let (rep, point) = {
        let log = lock(&session.rewind);
        match report(&log, dir, task) {
            Ok(rep) => (rep, log.get(task).cloned().unwrap_or_default()),
            Err(e) => return reply(0, false, false, e),
        }
    };
    if !confirm {
        return reply(rep.files, rep.outside, false, String::new());
    }
    if let Some(pre) = point.pre.as_ref().filter(|_| rep.files > 0) {
        // Pinned first, so the rewind is itself one "undo that" away.
        if let Err(e) = super::checkpoint::snapshot(dir, &format!("before rewind to #{task}")) {
            return reply(
                0,
                false,
                false,
                format!("could not pin the tree first: {e}"),
            );
        }
        if let Err(e) = super::checkpoint::restore(dir, pre) {
            return reply(0, false, false, format!("rewind failed: {e}"));
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
    *super::undo::lock(&session.undo) = None;
    lock(&session.rewind).drop_from(task);
    reply(rep.files, rep.outside, true, String::new())
}

#[cfg(test)]
#[path = "rewind_tests.rs"]
mod tests;

//! A follow-up in the same pane continues Claude Code's own session.
//!
//! Every message relayed to Claude Code was a fresh `claude -p`: what it had
//! read, run and changed the turn before was gone, and the follow-up got a
//! 2 KB recap of the words alone (`thread`). Asked "rename it and fix the
//! callers" after "where is relative_luminance used?", it searched the repo
//! again — 6 rounds, 5 calls, 18.8 s — where the same run resumed from the
//! first turn's session took 2 rounds, 1 call, 8.8 s (2026-10-30; a follow-up
//! that needed nothing from before was even, 22.8 s against 25.5 s). This is
//! Claude Code's own `--resume`, which its interactive sessions get for free.
//!
//! Only the first run of a new message resumes: the dials inside one task
//! carry their own transcript. A session older than [`FRESH_FOR`], one a
//! stop or a failure ended, and one another task already took are not
//! resumed, and a resume Claude Code refuses (a session it no longer has)
//! runs again fresh.
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use super::adapter::{HopStream, Usage};
use super::claudeagent::ClaudeAgent;

/// How long a session stays worth continuing.
const FRESH_FOR: Duration = Duration::from_secs(30 * 60);

/// The session the pane's last run ended on.
#[derive(Clone, Default)]
pub struct Resume(Arc<Mutex<Option<Last>>>);

struct Last {
    id: String,
    /// The task it ran for: its stop flag, one per task. Held, not its
    /// address — a finished task's flag is freed and the next one's can be
    /// given the same address, which read as the same task.
    task: Arc<AtomicBool>,
    at: Instant,
}

/// Which task a run belongs to: its stop flag.
pub(super) type Task = Option<Arc<AtomicBool>>;

impl Resume {
    /// The broker's one slot. The registry, and the adapter with it, is
    /// built again for every message (`Session::registry`); the pane — one
    /// broker process — is what has a conversation.
    pub(crate) fn shared() -> Self {
        static ONE: OnceLock<Resume> = OnceLock::new();
        ONE.get_or_init(Resume::default).clone()
    }

    /// The session a run of `task` may continue, taken so no other run
    /// continues it too: the last run's, when that ran for another task (so
    /// this is a new message) less than [`FRESH_FOR`] ago.
    pub(super) fn take_for(&self, task: &Task) -> Option<String> {
        let task = task.as_ref()?;
        let mut slot = self.0.lock().unwrap_or_else(|e| e.into_inner());
        let last = slot.take()?;
        if Arc::ptr_eq(&last.task, task) {
            *slot = Some(last); // a later dial of the same message
            return None;
        }
        (last.at.elapsed() < FRESH_FOR).then_some(last.id)
    }

    /// Keep the session a run of `task` ended on.
    pub(super) fn keep(&self, id: &str, task: &Task) {
        let (Some(task), false) = (task, id.is_empty()) else {
            return;
        };
        let last = Last {
            id: id.to_string(),
            task: Arc::clone(task),
            at: Instant::now(),
        };
        *self.0.lock().unwrap_or_else(|e| e.into_inner()) = Some(last);
    }

    /// Nothing to continue: the run was stopped or failed.
    pub(super) fn forget(&self) {
        *self.0.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }
}

impl ClaudeAgent {
    /// One message's run: the pane's last session continued when there is
    /// one to continue, and run again fresh when Claude Code refuses it.
    pub(super) fn run(
        &self,
        body: &str,
        t: Duration,
        s: &HopStream,
    ) -> Result<(String, Usage), String> {
        let task = super::cancelscope::current();
        let from = self.resume.take_for(&task);
        let got = match self.run_once(body, t, s, from.as_deref(), &task) {
            Err(e) if from.is_some() && e != super::cancelscope::STOPPED => {
                self.run_once(body, t, s, None, &task)
            }
            got => got,
        };
        if got.is_err() {
            self.resume.forget();
        }
        got
    }
}

#[cfg(all(test, unix))]
#[path = "clauderesume_tests.rs"]
mod tests;

//! The per-task memo around `toolchoice`, and the record the pane reads.
//!
//! WHY a memo: the engine asks the tool surface THREE times per task — `hint_for`,
//! `specs_for`, `note_for` (`crew_hive::apiagent`) — and a retried worker asks again with
//! the same text. A decision that costs a model call must be made once per question, not
//! once per read, so the answer is keyed on the task text (and the catalog's spelling) and
//! kept for a handful of tasks: a run's width and a retry or two, not a history.
//!
//! WHY a record: the choice is made INSIDE a worker (`Tools::hint_for`), which has no
//! emitter; the pane still deserves to know what agent smith decided. So the last decision
//! is kept here and `swarmcast::announcing` takes it, once per run. A child of `session`.
use std::sync::{Mutex, MutexGuard};

use super::toolchoice::{choose, disabled, label, live, ChooseFn, Chosen};
use crate::broker::toolpick::BUDGET;
use crate::mcp::McpTool;

/// Memo entries kept per session.
const MEMO_CAP: usize = 16;

/// Who decides. `Live` resolves the provider at the moment of need: a session is built long
/// before its first crowded task, and most sessions never have one. The other two are the
/// test seams — every production session is `Live`, and the switch is read at pick time.
#[cfg_attr(not(test), allow(dead_code))]
enum Mode {
    Live,
    Off,
    Fixed(Box<ChooseFn>),
}

/// The session's tool decider: the chooser, the per-task memo, and the last decision made,
/// held for the pane. One per pane (`Session::toolpick`), shared by every surface the
/// session builds, so a retried worker or a follow-up turn on the same text pays nothing.
pub(crate) struct Picker {
    mode: Mode,
    memo: Mutex<Vec<(String, Vec<McpTool>, usize)>>,
    last: Mutex<Option<Chosen>>,
}

impl Picker {
    pub(crate) fn live() -> Self {
        Self::with(Mode::Live)
    }
    /// Scorer only — the test surfaces, where a real call would be a network call.
    #[cfg(test)]
    pub(crate) fn off() -> Self {
        Self::with(Mode::Off)
    }
    /// An injected chooser — the seam the tests decide through.
    #[cfg(test)]
    pub(crate) fn fixed(f: Box<ChooseFn>) -> Self {
        Self::with(Mode::Fixed(f))
    }
    fn with(mode: Mode) -> Self {
        Self {
            mode,
            memo: Mutex::new(Vec::new()),
            last: Mutex::new(None),
        }
    }

    /// The tools `task` is shown and how many were left out: the memo's answer, else the
    /// model's (recorded for the pane), else the scorer's — each with the door.
    pub(crate) fn pick(&self, catalog: Vec<McpTool>, task: &str) -> (Vec<McpTool>, usize) {
        if catalog.len() <= BUDGET || disabled() {
            return super::toolselect::select(catalog, task);
        }
        let key = memo_key(&catalog, task);
        if let Some((_, kept, left)) = self.lock_memo().iter().find(|(k, _, _)| *k == key) {
            return (kept.clone(), *left);
        }
        let chosen = match &self.mode {
            Mode::Off => None,
            Mode::Fixed(f) => choose(&catalog, task, Some(&**f)),
            Mode::Live => live().and_then(|f| choose(&catalog, task, Some(&*f))),
        };
        let of = catalog.len();
        let result = match chosen {
            Some((kept, left)) => {
                let r = super::toolselect::with_door(kept, left);
                let names = r.0.iter().map(label).collect();
                *self.lock_last() = Some(Chosen { of, names });
                r
            }
            None => super::toolselect::select(catalog, task),
        };
        let mut memo = self.lock_memo();
        if memo.len() >= MEMO_CAP {
            memo.remove(0);
        }
        memo.push((key, result.0.clone(), result.1));
        result
    }

    /// The last model decision, taken once — the pane says it once per run.
    pub(crate) fn take_chosen(&self) -> Option<Chosen> {
        self.lock_last().take()
    }
    /// Forget the last decision: a run's start, so an earlier turn's is never announced.
    pub(crate) fn reset(&self) {
        *self.lock_last() = None;
    }
    fn lock_memo(&self) -> MutexGuard<'_, Vec<(String, Vec<McpTool>, usize)>> {
        self.memo.lock().unwrap_or_else(|e| e.into_inner())
    }
    fn lock_last(&self) -> MutexGuard<'_, Option<Chosen>> {
        self.last.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// The memo key: the task AND the catalog's spelling, so a server that connects mid-session
/// (`mcp.json` hot-reloads) makes the same task a new question.
fn memo_key(catalog: &[McpTool], task: &str) -> String {
    let mut k = catalog.iter().map(label).collect::<Vec<_>>().join(",");
    k.push('\n');
    k.push_str(task);
    k
}

#[cfg(test)]
#[path = "toolmemo_tests.rs"]
mod tests;

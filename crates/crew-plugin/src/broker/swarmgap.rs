//! A swarm where some tasks failed still answers from the ones that finished.
//!
//! WHY: the lead wrote its closing answer only when NOTHING failed. One
//! failed worker out of five and the turn ended on "swarm finished with 1
//! failed task"; the four good results sat in the worker rows above it,
//! and the user did the merging, and the finding, themselves. Now a run that
//! finished anything gets the answer (`swarmanswer::combine`), and its brief
//! names each task that has no result and why, so the answer says what is
//! missing instead of reading as complete. A run where nothing finished has
//! nothing to answer from and keeps the status line alone. A partial run is
//! never judged (`swarm::run_with_synth` says why). A child of `swarm`, so
//! `broker` items are reached through `crate::broker::`.
use std::collections::HashMap;

use crew_hive::{HiveEvent, TaskGraph, TaskId};

use super::swarmanswer::title;

/// A failure reason is a clause in the brief; a provider that rambles is cut.
const WHY_MAX: usize = 120;

/// Why each failed task failed, as the run's drain loop saw it. The
/// scheduler's outcome keeps only WHICH tasks failed (a failed result never
/// reaches the board), so the reason is caught off the bus: `AgentSpawned`
/// says which agent runs which task, and the agent's `Failed` carries the
/// error it showed in the pane.
#[derive(Default)]
pub(super) struct Reasons {
    task_of: HashMap<u64, TaskId>,
    why: HashMap<TaskId, String>,
}

impl Reasons {
    /// Fold one bus event in; anything but a spawn or a failure is ignored.
    pub(super) fn observe(&mut self, ev: &HiveEvent) {
        match ev {
            HiveEvent::AgentSpawned { agent, task } => {
                self.task_of.insert(agent.0, *task);
            }
            // The first error is the cause; anything after it is an echo.
            HiveEvent::Failed { agent, error } => {
                if let Some(task) = self.task_of.get(&agent.0) {
                    self.why.entry(*task).or_insert_with(|| error.clone());
                }
            }
            _ => {}
        }
    }

    /// The run's gap: every failed task's title, in the outcome's order, with
    /// its reason when one was seen. A panicked agent says nothing on the
    /// bus, and a lagged drain can miss what one did say; those go by title.
    pub(super) fn gap(&self, graph: &TaskGraph, failed: &[TaskId]) -> Gap {
        Gap(failed
            .iter()
            .map(|id| (title(graph, *id), self.why.get(id).cloned()))
            .collect())
    }
}

/// The tasks a run lost, each `(title, reason)`. Empty is a clean run.
#[derive(Default)]
pub(super) struct Gap(Vec<(String, Option<String>)>);

impl Gap {
    pub(super) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The closing brief's MISSING section. Empty on a clean run, so a clean
    /// brief is byte-for-byte what it was. It asks for the gap to be said
    /// outright: the brief above it says "no commentary about the process",
    /// and a missing part of the answer is not commentary.
    pub(super) fn brief(&self) -> String {
        if self.0.is_empty() {
            return String::new();
        }
        let named: Vec<String> = self
            .0
            .iter()
            .map(|(title, why)| match why {
                Some(w) => format!("\"{title}\" ({})", crate::broker::route::clip(w, WHY_MAX)),
                None => format!("\"{title}\""),
            })
            .collect();
        format!(
            "\n\nMISSING:\nThese sub-tasks FAILED and have no result: {}. Answer from what \
             finished, and say plainly what could not be done.",
            named.join(", ")
        )
    }
}

#[cfg(test)]
#[path = "swarmgap_tests.rs"]
mod run_tests;
#[cfg(test)]
#[path = "swarmgap_brief_tests.rs"]
mod tests;

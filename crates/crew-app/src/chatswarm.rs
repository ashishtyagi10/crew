//! Live swarm-run status for the chat pane: `HivePlan` opens a task-list
//! block, `Hive` telemetry updates it, and when every task reaches a terminal
//! state the block folds into a transcript message — the durable record of
//! the run. Rendering lives in `chatswarmview`.
use std::collections::HashMap;

use crew_hive::{HiveEvent, TaskId, TaskSpec, TaskState};

use crate::chat::ChatPane;
use crate::chatlayout::Message;

/// One planned task's live state in the block.
pub(crate) struct SwarmTask {
    pub id: TaskId,
    pub title: String,
    pub state: TaskState,
    /// Tokens spent by the agent running this task (input + output).
    pub tokens: u64,
}

/// The whole run's live state, built from `HivePlan` and fed by `Hive` events.
pub(crate) struct SwarmStatus {
    pub tasks: Vec<SwarmTask>,
    /// agent id → task id (from `AgentSpawned`) — `TokenDelta` only names agents.
    agent_task: HashMap<u64, TaskId>,
}

impl SwarmStatus {
    pub(crate) fn new(tasks: Vec<TaskSpec>) -> Self {
        SwarmStatus {
            tasks: tasks
                .into_iter()
                .map(|t| SwarmTask {
                    id: t.id,
                    title: t.title,
                    state: TaskState::Pending,
                    tokens: 0,
                })
                .collect(),
            agent_task: HashMap::new(),
        }
    }

    fn task_mut(&mut self, id: TaskId) -> Option<&mut SwarmTask> {
        self.tasks.iter_mut().find(|t| t.id == id)
    }

    pub(crate) fn apply(&mut self, ev: &HiveEvent) {
        match ev {
            HiveEvent::AgentSpawned { agent, task } => {
                self.agent_task.insert(agent.0, *task);
                if let Some(t) = self.task_mut(*task) {
                    t.state = TaskState::Running;
                }
            }
            HiveEvent::TaskStateChanged { task, state } => {
                if let Some(t) = self.task_mut(*task) {
                    t.state = *state;
                }
            }
            HiveEvent::TokenDelta {
                agent,
                input,
                output,
            } => {
                if let Some(&task) = self.agent_task.get(&agent.0) {
                    if let Some(t) = self.task_mut(task) {
                        t.tokens += u64::from(*input) + u64::from(*output);
                    }
                }
            }
            // Failed also arrives as TaskStateChanged(Failed); chunks/cost
            // land in the transcript via the broker's Message translation.
            HiveEvent::OutputChunk { .. }
            | HiveEvent::CostDelta { .. }
            | HiveEvent::Failed { .. } => {}
        }
    }

    /// Every task reached a terminal state.
    pub(crate) fn finished(&self) -> bool {
        self.tasks.iter().all(|t| {
            matches!(
                t.state,
                TaskState::Done | TaskState::Failed | TaskState::Cancelled
            )
        })
    }

    /// The block as a markdown list — the transcript record on fold.
    pub(crate) fn record_text(&self) -> String {
        self.tasks
            .iter()
            .map(|t| {
                let glyph = glyph(&t.state);
                if t.tokens > 0 {
                    format!("- {glyph} {} — {} tok", t.title, fmt_tok(t.tokens))
                } else {
                    format!("- {glyph} {}", t.title)
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Compact token count (`"12.4k"` past 1000) — shared by the live block
/// (`chatswarmview`) and the folded transcript record so the two never show
/// different numbers for the same run.
pub(crate) fn fmt_tok(n: u64) -> String {
    if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1000.0)
    } else {
        n.to_string()
    }
}

/// The state glyph shared by the live block and the folded record.
pub(crate) fn glyph(state: &TaskState) -> char {
    match state {
        TaskState::Pending | TaskState::Ready => '·',
        TaskState::Running => '⠿', // live view animates; record shows a static mark
        TaskState::Done => '✓',
        TaskState::Failed => '✗',
        TaskState::Cancelled => '⊘',
    }
}

impl ChatPane {
    /// A swarm plan landed: open (or reset) the live block.
    pub(crate) fn absorb_hive_plan(&mut self, tasks: Vec<TaskSpec>) {
        // A zero-task plan has no telemetry to fold it — never open a block
        // for one, or is_busy() would stay latched forever. The broker's
        // plan-summary and swarm-done messages already tell the story.
        if tasks.is_empty() {
            self.swarm = None;
            return;
        }
        self.swarm = Some(SwarmStatus::new(tasks));
    }

    /// Forwarded telemetry; folds the block once the run is over.
    pub(crate) fn absorb_hive(&mut self, ev: &HiveEvent) {
        let Some(s) = self.swarm.as_mut() else {
            return;
        };
        s.apply(ev);
        if s.finished() {
            self.fold_swarm();
        }
    }

    /// Retire the live block into a transcript message (the run's record).
    /// Also called on broker `Error` so a dead run leaves its partial state
    /// in the transcript instead of a forever-frozen block.
    pub(crate) fn fold_swarm(&mut self) {
        let Some(s) = self.swarm.take() else {
            return;
        };
        if self.scroll > 0 {
            self.unread += 1;
        }
        self.messages.push(Message {
            sender: "crew".into(),
            text: s.record_text(),
            ts: String::new(),
            meta: String::new(),
        });
        // Same drain the plugin's Message arm applies (chat.rs::poll) — the
        // folded record must not let the transcript grow past the cap.
        if self.messages.len() > 500 {
            let drain = self.messages.len() - 500;
            self.messages.drain(..drain);
        }
    }
}

#[cfg(test)]
#[path = "chatswarm_tests.rs"]
mod tests;

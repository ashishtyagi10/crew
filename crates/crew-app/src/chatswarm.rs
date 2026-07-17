//! Live swarm-run status for the chat pane: `HivePlan` opens the run's status
//! line, `Hive` telemetry updates it, and when every task reaches a terminal
//! state the line folds into a transcript message — the durable record of
//! the run. Live rendering (one status line: spinner, focused task, elapsed,
//! settled count) lives in `chatswarmview`; the folded record (task list + Σ
//! totals) in `chatswarmrec`.
use std::collections::HashMap;
use std::time::Instant;

use crew_hive::{HiveEvent, TaskId, TaskSpec, TaskState};

use crate::chat::ChatPane;
use crate::chatlayout::Message;

/// One planned task's live state in the block.
pub(crate) struct SwarmTask {
    pub id: TaskId,
    pub title: String,
    pub state: TaskState,
    /// Input tokens spent by the agent running this task.
    pub tokens_in: u64,
    /// Output tokens spent by the agent running this task.
    pub tokens_out: u64,
    /// Micro-USD spent by the agent running this task (`CostDelta`) — 0 for
    /// stub/keyless runs, which never emit cost.
    pub cost_micros: u64,
    /// When the task started running — stamped once by whichever of
    /// `AgentSpawned`/`TaskStateChanged(Running)` arrives first. `None` until
    /// then (and forever, if the task is cancelled before either arrives).
    pub started: Option<Instant>,
    /// Duration captured when a terminal state is reached (`started.elapsed()`
    /// at that moment). `None` if the task never started.
    pub elapsed_ms: Option<u64>,
}

impl SwarmTask {
    /// Total tokens spent on this task — input plus output. The folded record
    /// (`chatswarmrec`) reports one combined figure per task; the live line
    /// (`chatswarmview`) reports the run's `↑in ↓out` split via
    /// [`SwarmStatus::token_totals`].
    pub(crate) fn tokens(&self) -> u64 {
        self.tokens_in + self.tokens_out
    }
}

/// The whole run's live state, built from `HivePlan` and fed by `Hive` events.
pub(crate) struct SwarmStatus {
    pub tasks: Vec<SwarmTask>,
    /// agent id → task id (from `AgentSpawned`) — `TokenDelta` only names agents.
    agent_task: HashMap<u64, TaskId>,
    /// When the plan arrived — the zero point for the run's wall-clock
    /// duration on the folded record's Σ line (`chatswarmrec`).
    pub(crate) run_started: Instant,
}

impl SwarmStatus {
    pub(crate) fn new(tasks: Vec<TaskSpec>) -> Self {
        SwarmStatus {
            run_started: Instant::now(),
            tasks: tasks
                .into_iter()
                .map(|t| SwarmTask {
                    id: t.id,
                    title: t.title,
                    state: TaskState::Pending,
                    tokens_in: 0,
                    tokens_out: 0,
                    cost_micros: 0,
                    started: None,
                    elapsed_ms: None,
                })
                .collect(),
            agent_task: HashMap::new(),
        }
    }

    fn task_mut(&mut self, id: TaskId) -> Option<&mut SwarmTask> {
        self.tasks.iter_mut().find(|t| t.id == id)
    }

    /// `(settled, total)` — tasks that have reached a terminal state, over the
    /// plan's size. Terminal means done, failed or cancelled: this counts "how
    /// much of the plan has stopped moving", not "how much succeeded".
    ///
    /// Shared by the progress bar (`chatprog`) and the live status line
    /// (`chatswarmview`) so the bar's fill and the line's `2/5` can never
    /// disagree about the same run.
    pub(crate) fn settled(&self) -> (usize, usize) {
        let done = self
            .tasks
            .iter()
            .filter(|t| {
                matches!(
                    t.state,
                    TaskState::Done | TaskState::Failed | TaskState::Cancelled
                )
            })
            .count();
        (done, self.tasks.len())
    }

    /// `(input, output)` tokens summed across the whole run — the live line's
    /// `↑in ↓out`. The per-task split arrives on `TokenDelta`; this rolls it
    /// up so the status line can show the run's spend at a glance.
    pub(crate) fn token_totals(&self) -> (u64, u64) {
        self.tasks
            .iter()
            .fold((0, 0), |(i, o), t| (i + t.tokens_in, o + t.tokens_out))
    }

    pub(crate) fn apply(&mut self, ev: &HiveEvent) {
        match ev {
            HiveEvent::AgentSpawned { agent, task } => {
                self.agent_task.insert(agent.0, *task);
                if let Some(t) = self.task_mut(*task) {
                    t.state = TaskState::Running;
                    t.started.get_or_insert_with(Instant::now);
                }
            }
            HiveEvent::TaskStateChanged { task, state } => {
                if let Some(t) = self.task_mut(*task) {
                    t.state = *state;
                    if *state == TaskState::Running {
                        t.started.get_or_insert_with(Instant::now);
                    } else if matches!(
                        state,
                        TaskState::Done | TaskState::Failed | TaskState::Cancelled
                    ) {
                        t.elapsed_ms = t.started.map(|s| s.elapsed().as_millis() as u64);
                    }
                }
            }
            HiveEvent::TokenDelta {
                agent,
                input,
                output,
            } => {
                if let Some(&task) = self.agent_task.get(&agent.0) {
                    if let Some(t) = self.task_mut(task) {
                        t.tokens_in += u64::from(*input);
                        t.tokens_out += u64::from(*output);
                    }
                }
            }
            HiveEvent::CostDelta { agent, micros_usd } => {
                if let Some(&task) = self.agent_task.get(&agent.0) {
                    if let Some(t) = self.task_mut(task) {
                        t.cost_micros += *micros_usd;
                    }
                }
            }
            // Failed also arrives as TaskStateChanged(Failed); chunks land in
            // the transcript via the broker's Message translation.
            HiveEvent::OutputChunk { .. } | HiveEvent::Failed { .. } => {}
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
        let run_ms = s.run_started.elapsed().as_millis() as u64;
        self.push_capped(Message {
            sender: "crew".into(),
            text: s.record_text(Some(run_ms)),
            ts: String::new(),
            meta: String::new(),
        });
    }
}

#[cfg(test)]
#[path = "chatswarm_tests.rs"]
mod tests;

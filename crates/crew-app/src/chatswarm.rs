//! Live swarm-run status for the chat pane: `HivePlan` opens the run's block,
//! `Hive` telemetry updates it, and when every task reaches a terminal state
//! the block folds into one transcript record (`chatswarmrec`). Live
//! rendering — the status line (`chatswarmview`) over one row per task with
//! its span on a shared clock (`chatswarmrows`, `chatswarmspan`) — reads this
//! model; the LOG tee of the same events is `chatswarmlog`.
use std::collections::HashMap;
use std::time::Instant;

use crew_hive::{HiveEvent, TaskId, TaskSpec, TaskState};

/// One planned task's live state in the block.
pub(crate) struct SwarmTask {
    pub id: TaskId,
    pub title: String,
    /// The `@`-handle of the specialist the plan gave the task; may be empty.
    pub specialty: String,
    /// What it waits on, as the plan said it — the rows point at these.
    pub deps: Vec<TaskId>,
    pub state: TaskState,
    /// Input tokens spent by the agent running this task.
    pub tokens_in: u64,
    /// Output tokens spent by the agent running this task.
    pub tokens_out: u64,
    /// When the task started running — stamped once by whichever of
    /// `AgentSpawned`/`TaskStateChanged(Running)` arrives first. `None` until
    /// then (and forever, if the task is cancelled before either arrives).
    /// Drives the live line's focused-task ordering and elapsed readout.
    pub started: Option<Instant>,
    /// The same moment on the frame clock (`anim::now_ms`), for the span bar
    /// — an `Instant` cannot be placed on a frame's axis, and tests stamp
    /// this one by hand.
    pub started_ms: Option<u64>,
    /// When it left `Running`, on the same clock; `None` while it runs.
    pub ended_ms: Option<u64>,
}

/// Done, failed or cancelled: the task has stopped moving.
pub(crate) fn terminal(state: TaskState) -> bool {
    matches!(
        state,
        TaskState::Done | TaskState::Failed | TaskState::Cancelled
    )
}

/// The whole run's live state, built from `HivePlan` and fed by `Hive` events.
pub(crate) struct SwarmStatus {
    pub tasks: Vec<SwarmTask>,
    /// agent id → task id (from `AgentSpawned`) — `TokenDelta` only names agents.
    agent_task: HashMap<u64, TaskId>,
    /// The progress bar's sweeping fill (`chatprogspring`); born settled.
    pub(crate) fill: crate::readout::Counter,
    /// The status line's counter flash (`chatflash`); born quiet.
    pub(crate) flash: crate::chatflash::Flash,
    /// The run's tool pool, `(used, total)`, as of the latest draw
    /// (`HiveEvent::ToolBudget`) or the aggregate Stats; `None` until either.
    pub tools: Option<(u32, u32)>,
}

impl SwarmStatus {
    pub(crate) fn new(tasks: Vec<TaskSpec>) -> Self {
        SwarmStatus {
            tasks: tasks
                .into_iter()
                .map(|t| SwarmTask {
                    id: t.id,
                    title: t.title,
                    specialty: t.specialty,
                    deps: t.deps,
                    state: TaskState::Pending,
                    tokens_in: 0,
                    tokens_out: 0,
                    started: None,
                    started_ms: None,
                    ended_ms: None,
                })
                .collect(),
            agent_task: HashMap::new(),
            fill: Default::default(),
            flash: Default::default(),
            tools: None,
        }
    }

    fn task_mut(&mut self, id: TaskId) -> Option<&mut SwarmTask> {
        self.tasks.iter_mut().find(|t| t.id == id)
    }

    /// `(settled, total)` — done, failed or cancelled tasks over the plan's
    /// size: "how much has stopped moving", not "how much succeeded". Shared by
    /// the bar (`chatprog`) and the line (`chatswarmview`) so they never disagree.
    pub(crate) fn settled(&self) -> (usize, usize) {
        let done = self.tasks.iter().filter(|t| terminal(t.state)).count();
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

    /// [`Self::apply_at`] on the frame clock.
    pub(crate) fn apply(&mut self, ev: &HiveEvent) {
        self.apply_at(ev, crate::anim::now_ms());
    }

    /// Fold one event in, stamping any start or end it implies at `now_ms`.
    pub(crate) fn apply_at(&mut self, ev: &HiveEvent, now_ms: u64) {
        match ev {
            HiveEvent::AgentSpawned { agent, task } => {
                self.agent_task.insert(agent.0, *task);
                if let Some(t) = self.task_mut(*task) {
                    t.state = TaskState::Running;
                    t.started.get_or_insert_with(Instant::now);
                    t.started_ms.get_or_insert(now_ms);
                    t.ended_ms = None;
                }
            }
            HiveEvent::TaskStateChanged { task, state } => {
                if let Some(t) = self.task_mut(*task) {
                    t.state = *state;
                    if *state == TaskState::Running {
                        t.started.get_or_insert_with(Instant::now);
                        t.started_ms.get_or_insert(now_ms);
                        t.ended_ms = None;
                    } else if terminal(*state) && t.started_ms.is_some() && t.ended_ms.is_none() {
                        t.ended_ms = Some(now_ms);
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
            HiveEvent::ToolBudget { used, total } => self.tools = Some((*used, *total)),
            // Not this block's: cost; chunks (the broker's Message); deltas
            // (`chatflow`/`chatthought`); tools (`chattool`); Failed (a state).
            HiveEvent::CostDelta { .. }
            | HiveEvent::OutputChunk { .. }
            | HiveEvent::OutputDelta { .. }
            | HiveEvent::ThoughtDelta { .. }
            | HiveEvent::ToolCall { .. }
            | HiveEvent::ToolResult { .. }
            | HiveEvent::Loaded { .. }
            | HiveEvent::Failed { .. } => {}
        }
    }

    /// Every task reached a terminal state.
    pub(crate) fn finished(&self) -> bool {
        self.tasks.iter().all(|t| terminal(t.state))
    }
}

#[cfg(test)]
#[path = "chatswarm_tests.rs"]
mod tests;

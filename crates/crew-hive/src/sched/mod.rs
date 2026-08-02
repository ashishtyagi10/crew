//! Scheduler: runs a `TaskGraph` to completion over a bounded pool of agents.
//! Ready tasks (deps all done) are spawned onto a `JoinSet`, each gated by a
//! `Semaphore` permit (the concurrency cap). Results land in the `Blackboard`;
//! state transitions emit on the `EventBus`; a failed/cancelled task cascades
//! cancellation to its dependents.
//!
//! Cooperative cancellation: call `.with_cancel(flag)` before `.run()`. When
//! the flag is set, the scheduler stops spawning new tasks, marks all
//! unstarted tasks `Cancelled`, and drains in-flight agents to completion.
mod cancel;
mod replan;
#[cfg(test)]
mod tests;

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use futures::FutureExt as _;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::agent::{AgentContext, AgentFactory};
use crate::board::Blackboard;
use crate::bus::{AgentId, EventBus, HiveEvent};
use crate::graph::{TaskGraph, TaskId, TaskState};

use cancel::{
    cascade_cancel, mark_all_unstarted_cancelled, record_cancelled, record_result, sorted,
};

#[derive(Clone, Debug, PartialEq)]
pub struct RunOutcome {
    pub done: Vec<TaskId>,
    pub failed: Vec<TaskId>,
    pub cancelled: Vec<TaskId>,
}

pub struct Scheduler {
    graph: TaskGraph,
    board: Blackboard,
    bus: EventBus,
    factory: Arc<dyn AgentFactory>,
    concurrency: usize,
    cancel: Arc<AtomicBool>,
    /// When set, the first task failure triggers ONE re-plan of the
    /// not-yet-run remainder (see [`replan`]); `None` — keyless, mock, or a
    /// host that never opted in — keeps pure cascade-cancel.
    replan: Option<replan::Replan>,
}

impl Scheduler {
    pub fn new(
        graph: TaskGraph,
        board: Blackboard,
        bus: EventBus,
        factory: Arc<dyn AgentFactory>,
        concurrency: usize,
    ) -> Self {
        Self {
            graph,
            board,
            bus,
            factory,
            concurrency: concurrency.max(1),
            cancel: Arc::new(AtomicBool::new(false)),
            replan: None,
        }
    }

    /// Allow one mid-run re-plan (builder-style): on the first failure the
    /// planner is asked for a replacement of the not-yet-run remainder,
    /// capped at [`replan::REPLAN_CAP`] per run.
    pub fn with_replan(
        mut self,
        goal: impl Into<String>,
        planner: Arc<dyn crate::planner::Planner>,
    ) -> Self {
        self.replan = Some(replan::Replan {
            goal: goal.into(),
            planner,
        });
        self
    }

    /// Attach a shared cancel flag (builder-style). When the flag is set,
    /// the scheduler stops spawning new tasks and cancels all unstarted
    /// tasks, but drains the `JoinSet` so in-flight agents finish normally.
    pub fn with_cancel(mut self, cancel: Arc<AtomicBool>) -> Self {
        self.cancel = cancel;
        self
    }

    pub async fn run(self) -> RunOutcome {
        let sem = Arc::new(Semaphore::new(self.concurrency));
        // The graph is LOCAL and mutable: a re-plan swaps the not-yet-run
        // remainder for a replacement sub-graph mid-run.
        let mut graph = self.graph.clone();
        let mut replans_left = replan::REPLAN_CAP;
        let mut done: HashSet<TaskId> = HashSet::new();
        let mut failed: HashSet<TaskId> = HashSet::new();
        let mut cancelled: HashSet<TaskId> = HashSet::new();
        let mut started: HashSet<TaskId> = HashSet::new();
        // `None` = the task was cancelled while queued for a permit, so its
        // agent never ran: that is a cancellation, not a failure, and must not
        // cascade to dependents as one.
        let mut joinset: JoinSet<(TaskId, Option<crate::board::TaskResult>)> = JoinSet::new();
        let mut next_agent: u64 = 0;

        loop {
            // --- Cooperative cancellation check ---
            if self.cancel.load(Ordering::Relaxed) {
                mark_all_unstarted_cancelled(
                    &graph,
                    &self.bus,
                    &done,
                    &failed,
                    &mut cancelled,
                    &started,
                );
                // Drain all in-flight agents; never abort running work. Ones
                // still queued for a permit bail out and land here as `None`.
                while let Some(joined) = joinset.join_next().await {
                    let (id, result) = joined.expect("agent task panicked");
                    match result {
                        Some(r) => {
                            record_result(id, r, &mut done, &mut failed, &self.board, &self.bus)
                                .await
                        }
                        None => record_cancelled(id, &mut cancelled, &self.bus),
                    }
                }
                break;
            }

            cascade_cancel(&graph, &self.bus, &done, &failed, &mut cancelled, &started);

            // Spawn every ready (deps all done), not-yet-started task.
            for id in graph.ready(&done) {
                if started.contains(&id) || cancelled.contains(&id) {
                    continue;
                }
                started.insert(id);
                let spec = graph.get(id).unwrap().clone();
                let agent_id = AgentId(next_agent);
                next_agent += 1;
                let agent = self.factory.make(&spec.agent);
                let bus = self.bus.clone();
                let board = self.board.clone();
                let sem = sem.clone();
                let cancel = self.cancel.clone();
                joinset.spawn(async move {
                    let task_id = spec.id;
                    let _permit = sem.acquire_owned().await.expect("semaphore open");
                    // `started` is stamped at SPAWN, but with a concurrency cap
                    // most spawned tasks then sit here waiting for a permit —
                    // queued, not running. So `mark_all_unstarted_cancelled`
                    // skips them, and without this check each one would take
                    // its turn and run a full agent after the user pressed
                    // stop (or the budget cap tripped) — billing them for work
                    // they cancelled. A task only becomes un-cancellable once
                    // its agent is actually running.
                    if cancel.load(Ordering::Relaxed) {
                        return (task_id, None);
                    }
                    let deps = board.gather(&spec.deps).await;
                    bus.publish(HiveEvent::AgentSpawned {
                        agent: agent_id.clone(),
                        task: spec.id,
                    });
                    bus.publish(HiveEvent::TaskStateChanged {
                        task: spec.id,
                        state: TaskState::Running,
                    });
                    let ctx = AgentContext {
                        agent: agent_id,
                        task: spec,
                        deps,
                        bus,
                    };
                    let result = match std::panic::AssertUnwindSafe(agent.run(ctx))
                        .catch_unwind()
                        .await
                    {
                        Ok(r) => r,
                        Err(_) => crate::board::TaskResult {
                            task: task_id,
                            output: "agent panicked".into(),
                            success: false,
                        },
                    };
                    (task_id, Some(result))
                });
            }

            if joinset.is_empty() {
                break;
            }

            if let Some(joined) = joinset.join_next().await {
                let (id, result) = joined.expect("agent task panicked");
                match result {
                    Some(r) => {
                        let failure = (!r.success).then(|| r.output.clone());
                        record_result(id, r, &mut done, &mut failed, &self.board, &self.bus).await;
                        // A failure burns the run's one re-plan attempt (see
                        // `replan::attempt`, awaited before the next spawn
                        // pass — the pause).
                        if let (Some(f), Some(rp)) = (failure, &self.replan) {
                            if replans_left > 0 {
                                replans_left -= 1;
                                #[rustfmt::skip]
                                replan::attempt(
                                    rp, &mut graph, &self.board, &self.bus,
                                    &done, &failed, &mut cancelled, &started,
                                    id, &f,
                                ).await;
                            }
                        }
                    }
                    None => record_cancelled(id, &mut cancelled, &self.bus),
                }
            }
        }

        RunOutcome {
            done: sorted(done),
            failed: sorted(failed),
            cancelled: sorted(cancelled),
        }
    }
}

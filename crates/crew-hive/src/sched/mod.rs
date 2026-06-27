//! Scheduler: runs a `TaskGraph` to completion over a bounded pool of agents.
//! Ready tasks (deps all done) are spawned onto a `JoinSet`, each gated by a
//! `Semaphore` permit (the concurrency cap). Results land in the `Blackboard`;
//! state transitions emit on the `EventBus`; a failed/cancelled task cascades
//! cancellation to its dependents.
#[cfg(test)]
mod tests;

use std::collections::HashSet;
use std::sync::Arc;

use futures::FutureExt as _;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::agent::{AgentContext, AgentFactory};
use crate::board::Blackboard;
use crate::bus::{AgentId, EventBus, HiveEvent};
use crate::graph::{TaskGraph, TaskId, TaskState};

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
        }
    }

    pub async fn run(self) -> RunOutcome {
        let sem = Arc::new(Semaphore::new(self.concurrency));
        let mut done: HashSet<TaskId> = HashSet::new();
        let mut failed: HashSet<TaskId> = HashSet::new();
        let mut cancelled: HashSet<TaskId> = HashSet::new();
        let mut started: HashSet<TaskId> = HashSet::new();
        let mut joinset: JoinSet<(TaskId, crate::board::TaskResult)> = JoinSet::new();
        let mut next_agent: u64 = 0;

        loop {
            cascade_cancel(
                &self.graph,
                &self.bus,
                &done,
                &failed,
                &mut cancelled,
                &started,
            );
            // Spawn every ready (deps all done), not-yet-started task.
            for id in self.graph.ready(&done) {
                if started.contains(&id) || cancelled.contains(&id) {
                    continue;
                }
                started.insert(id);
                let spec = self.graph.get(id).unwrap().clone();
                let agent_id = AgentId(next_agent);
                next_agent += 1;
                let agent = self.factory.make(&spec.agent);
                let bus = self.bus.clone();
                let board = self.board.clone();
                let sem = sem.clone();
                joinset.spawn(async move {
                    let _permit = sem.acquire_owned().await.expect("semaphore open");
                    let deps = board.gather(&spec.deps).await;
                    bus.publish(HiveEvent::AgentSpawned {
                        agent: agent_id.clone(),
                        task: spec.id,
                    });
                    bus.publish(HiveEvent::TaskStateChanged {
                        task: spec.id,
                        state: TaskState::Running,
                    });
                    let task_id = spec.id;
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
                    (task_id, result)
                });
            }

            if joinset.is_empty() {
                break;
            }

            if let Some(joined) = joinset.join_next().await {
                // Panics are caught inside the task and converted to a failed TaskResult,
                // so a JoinError here only occurs on abort — which this scheduler never does.
                let (id, result) = joined.expect("agent task panicked");
                if result.success {
                    self.board.put_result(result).await;
                    done.insert(id);
                    self.bus.publish(HiveEvent::TaskStateChanged {
                        task: id,
                        state: TaskState::Done,
                    });
                } else {
                    failed.insert(id);
                    self.bus.publish(HiveEvent::TaskStateChanged {
                        task: id,
                        state: TaskState::Failed,
                    });
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

/// Mark every not-started task with a failed/cancelled dependency as
/// cancelled (transitively, since newly-cancelled tasks feed the next pass
/// via the scheduler loop).
fn cascade_cancel(
    graph: &TaskGraph,
    bus: &EventBus,
    done: &HashSet<TaskId>,
    failed: &HashSet<TaskId>,
    cancelled: &mut HashSet<TaskId>,
    started: &HashSet<TaskId>,
) {
    loop {
        let mut changed = false;
        for t in graph.tasks() {
            if done.contains(&t.id)
                || failed.contains(&t.id)
                || cancelled.contains(&t.id)
                || started.contains(&t.id)
            {
                continue;
            }
            if t.deps
                .iter()
                .any(|d| failed.contains(d) || cancelled.contains(d))
            {
                cancelled.insert(t.id);
                bus.publish(HiveEvent::TaskStateChanged {
                    task: t.id,
                    state: TaskState::Cancelled,
                });
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
}

fn sorted(set: HashSet<TaskId>) -> Vec<TaskId> {
    let mut v: Vec<TaskId> = set.into_iter().collect();
    v.sort_unstable();
    v
}

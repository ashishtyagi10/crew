//! Bookkeeping helpers for the scheduler loop: cascade dependency
//! cancellation, result/cancel recording, and sort utilities.

use crate::board::Blackboard;
use crate::bus::{EventBus, HiveEvent};
use crate::graph::{TaskGraph, TaskId, TaskState};
use std::collections::HashSet;

/// Mark every not-started task with a failed/cancelled dependency as
/// cancelled (transitively, since newly-cancelled tasks feed the next pass
/// via the scheduler loop).
pub(super) fn cascade_cancel(
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

/// Mark every not-yet-cancelled, not-started, not-done, not-failed task
/// as `Cancelled`. Used when the cancel flag fires.
pub(super) fn mark_all_unstarted_cancelled(
    graph: &TaskGraph,
    bus: &EventBus,
    done: &HashSet<TaskId>,
    failed: &HashSet<TaskId>,
    cancelled: &mut HashSet<TaskId>,
    started: &HashSet<TaskId>,
) {
    for t in graph.tasks() {
        if done.contains(&t.id)
            || failed.contains(&t.id)
            || cancelled.contains(&t.id)
            || started.contains(&t.id)
        {
            continue;
        }
        cancelled.insert(t.id);
        bus.publish(HiveEvent::TaskStateChanged {
            task: t.id,
            state: TaskState::Cancelled,
        });
    }
}

pub(super) fn sorted(set: HashSet<TaskId>) -> Vec<TaskId> {
    let mut v: Vec<TaskId> = set.into_iter().collect();
    v.sort_unstable();
    v
}

/// A task that bailed at the permit gate: its agent never ran, so it is
/// cancelled — not failed. `cascade_cancel` already treats cancelled and
/// failed dependents alike, but the run's OUTCOME must not report work the
/// user stopped as work that broke.
pub(super) fn record_cancelled(id: TaskId, cancelled: &mut HashSet<TaskId>, bus: &EventBus) {
    cancelled.insert(id);
    bus.publish(HiveEvent::TaskStateChanged {
        task: id,
        state: TaskState::Cancelled,
    });
}

pub(super) async fn record_result(
    id: TaskId,
    result: crate::board::TaskResult,
    done: &mut HashSet<TaskId>,
    failed: &mut HashSet<TaskId>,
    board: &Blackboard,
    bus: &EventBus,
) {
    if result.success {
        board.put_result(result).await;
        done.insert(id);
        bus.publish(HiveEvent::TaskStateChanged {
            task: id,
            state: TaskState::Done,
        });
    } else {
        failed.insert(id);
        bus.publish(HiveEvent::TaskStateChanged {
            task: id,
            state: TaskState::Failed,
        });
    }
}

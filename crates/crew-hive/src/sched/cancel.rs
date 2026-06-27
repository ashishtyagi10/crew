//! Cancel helpers: cascade dependency cancellation and sort utilities.

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

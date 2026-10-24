//! How a task's turn ended, as the scheduler hears it.

use crate::agent::Attempt;
use crate::board::TaskResult;
use crate::graph::TaskId;
use crate::harness::Ended;

/// How a turn ended, as the scheduler hears it for `task`: an answer is a
/// success, a failure carries its reason as the output and whether it
/// passes, and a stop is recorded cancelled, never failed.
pub(super) fn attempt_of(ended: Ended, task: TaskId) -> Attempt {
    match ended {
        Ended::Answered(output) => TaskResult {
            task,
            output,
            success: true,
        }
        .into(),
        Ended::Failed { why, transient } => Attempt {
            result: TaskResult {
                task,
                output: why,
                success: false,
            },
            transient,
            stopped: false,
        },
        Ended::Stopped => Attempt::stopped(task),
    }
}

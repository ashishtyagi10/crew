//! JoinSet bookkeeping: a task panic must become a failed task, never abort the run.
use std::collections::HashMap;

use tokio::task::{Id, JoinSet};

use crate::board::TaskResult;
use crate::graph::TaskId;

type Work = (TaskId, Option<TaskResult>);

/// Consume one completion; an unexpected JoinError is attached to its task id.
pub(super) async fn next(
    tasks: &mut JoinSet<Work>,
    ids: &mut HashMap<Id, TaskId>,
) -> Option<Work> {
    match tasks.join_next_with_id().await? {
        Ok((id, work)) => {
            ids.remove(&id)?;
            Some(work)
        }
        Err(error) => {
            let task = ids.remove(&error.id())?;
            Some((
                task,
                Some(TaskResult {
                    task,
                    output: "agent task panicked".into(),
                    success: false,
                }),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn join_error_becomes_a_failed_tracked_task() {
        let mut tasks: JoinSet<Work> = JoinSet::new();
        let mut ids = HashMap::new();
        let id = TaskId(7);
        let abort = tasks.spawn(async { panic!("unexpected task panic") });
        assert!(ids.insert(abort.id(), id).is_none());

        let (joined, result) = next(&mut tasks, &mut ids).await.unwrap();
        assert_eq!(joined, id);
        let result = result.unwrap();
        assert_eq!(result.task, id);
        assert!(!result.success);
        assert_eq!(result.output, "agent task panicked");
        assert!(ids.is_empty());
    }
}

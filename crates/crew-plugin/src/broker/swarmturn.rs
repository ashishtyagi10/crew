//! What a swarm turn leaves in the pane's thread (`broker::thread`): the one
//! answer the user read. That is the lead's closing answer when one was
//! written (`swarmanswer::combine`); otherwise the sinks' own replies WERE
//! the answer — one sink, its output whole; several, each under its title
//! and held to the same budgets every reader of a run's outputs uses. A
//! child of `swarm`, so `broker` items are reached through `crate::broker::`.
use crew_hive::{TaskGraph, TaskResult};

use super::swarmanswer::{outputs, parts};

/// The answer to record, or `None` when the run produced nothing to show —
/// which is not a turn (a failed or cancelled run never reaches here).
pub(super) fn reply(
    answer: Option<String>,
    graph: &TaskGraph,
    results: &[TaskResult],
) -> Option<String> {
    if answer.is_some() {
        return answer;
    }
    let is_sink = |r: &&TaskResult| !graph.tasks().iter().any(|s| s.deps.contains(&r.task));
    let sinks = parts(graph, results.iter().filter(is_sink));
    match sinks.as_slice() {
        [] => None,
        [(_, one)] => Some(one.clone()).filter(|s| !s.trim().is_empty()),
        _ => Some(outputs(&sinks).trim().to_owned()),
    }
}

#[cfg(test)]
#[path = "swarmturn_tests.rs"]
mod tests;

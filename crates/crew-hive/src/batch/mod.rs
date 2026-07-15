//! Batch graph builder: turns a flat list of `Job`s into an independent
//! `TaskGraph` (no deps between jobs — all run in parallel up to the
//! scheduler's concurrency cap).
#[cfg(test)]
mod tests;

use crate::graph::{AgentKind, GraphError, ModelTier, TaskGraph, TaskId, TaskSpec};

/// A single batch job: title, prompt, and the model-cost tier to use.
#[derive(Clone, Debug)]
pub struct Job {
    pub title: String,
    pub prompt: String,
    pub tier: ModelTier,
}

/// Build a flat `TaskGraph` from a list of `Job`s.
///
/// Every job gets a unique `TaskId` (`i as u64` for index `i`). No
/// dependency edges are added — all tasks are roots and become ready
/// immediately. An empty input produces an empty graph.
pub fn batch_graph(jobs: Vec<Job>) -> Result<TaskGraph, GraphError> {
    let specs: Vec<TaskSpec> = jobs
        .into_iter()
        .enumerate()
        .map(|(i, job)| TaskSpec {
            id: TaskId(i as u64),
            title: job.title,
            agent: AgentKind::Api { system: None },
            model: job.tier,
            deps: vec![],
            prompt: job.prompt,
            specialty: String::new(),
            expertise: String::new(),
        })
        .collect();
    TaskGraph::new(specs)
}

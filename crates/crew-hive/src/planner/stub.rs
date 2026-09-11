//! The deterministic planner: `fanout` leaves plus one merge task, no model.
//!
//! WHY its own file: it is the planner every keyless and mock swarm runs on
//! (`swarmconf::backend`), and every scheduler test drives — so it lives
//! apart from the prompt in `mod.rs`, which is the LLM planner's and is held
//! to the line cap.
use std::future::Future;
use std::pin::Pin;

use super::{PlanError, Planner};
use crate::graph::{AgentKind, ModelTier, TaskGraph, TaskId, TaskSpec};

/// Deterministic planner for tests: builds `fanout` leaf tasks plus one merge
/// task that depends on all of them. No LLM required. Every task's prompt is
/// the goal verbatim, which is what lets a test read the framed goal off the
/// `HivePlan` event.
pub struct StubPlanner {
    pub fanout: usize,
}

impl Planner for StubPlanner {
    fn plan(
        &self,
        goal: &str,
    ) -> Pin<Box<dyn Future<Output = Result<TaskGraph, PlanError>> + Send>> {
        let fanout = self.fanout;
        let goal = goal.to_owned();
        Box::pin(async move {
            let mut tasks: Vec<TaskSpec> = (0..fanout)
                .map(|i| TaskSpec {
                    id: TaskId(i as u64),
                    title: format!("leaf-{i}"),
                    agent: AgentKind::Api { system: None },
                    model: ModelTier::Standard,
                    deps: vec![],
                    prompt: goal.clone(),
                    specialty: format!("leaf-{i}"),
                    expertise: String::new(),
                })
                .collect();
            let merge = TaskSpec {
                id: TaskId(fanout as u64),
                title: "merge".into(),
                agent: AgentKind::Api { system: None },
                model: ModelTier::Standard,
                deps: (0..fanout).map(|i| TaskId(i as u64)).collect(),
                prompt: goal,
                specialty: "merge".into(),
                expertise: String::new(),
            };
            tasks.push(merge);
            Ok(TaskGraph::new(tasks)?)
        })
    }
}

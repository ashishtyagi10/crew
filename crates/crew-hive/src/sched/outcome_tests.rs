use crate::agent::StubFactory;
use crate::board::Blackboard;
use crate::bus::EventBus;
use crate::graph::{AgentKind, ModelTier, TaskGraph, TaskId, TaskSpec};
use crate::sched::Scheduler;
use std::sync::Arc;

fn spec(id: u64, deps: &[u64]) -> TaskSpec {
    TaskSpec {
        id: TaskId(id),
        title: format!("t{id}"),
        agent: AgentKind::Api { system: None },
        model: ModelTier::Standard,
        deps: deps.iter().map(|d| TaskId(*d)).collect(),
        prompt: String::new(),
        specialty: String::new(),
        expertise: String::new(),
    }
}

#[tokio::test]
async fn the_outcome_reports_a_pool_sized_four_per_task_and_untouched_by_agents_without_tools() {
    let g = TaskGraph::new(vec![spec(0, &[]), spec(1, &[0]), spec(2, &[0])]).unwrap();
    let sched = Scheduler::new(
        g,
        Blackboard::new(),
        EventBus::new(64),
        Arc::new(StubFactory),
        4,
    );
    let out = sched.run().await;
    assert_eq!(out.done.len(), 3);
    assert_eq!(out.tool_rounds, (0, 12), "stub agents never draw a round");
}

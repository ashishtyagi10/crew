use super::*;
use crate::board::TaskResult;
use crate::bus::{AgentId, EventBus, HiveEvent};
use crate::graph::{AgentKind, ModelTier, TaskId, TaskSpec};
use std::collections::HashSet;

fn spec(id: u64) -> TaskSpec {
    TaskSpec {
        id: TaskId(id),
        title: "t".into(),
        agent: AgentKind::Api { system: None },
        model: ModelTier::Standard,
        deps: vec![],
        prompt: String::new(),
        specialty: String::new(),
        expertise: String::new(),
    }
}

#[tokio::test]
async fn stub_agent_succeeds_and_emits() {
    let bus = EventBus::new(32);
    let mut rx = bus.subscribe();
    let agent = StubAgent {
        fail_ids: HashSet::new(),
    };
    let ctx = AgentContext {
        agent: AgentId(0),
        task: spec(7),
        deps: vec![TaskResult {
            task: TaskId(1),
            output: "d".into(),
            success: true,
        }],
        bus: bus.clone(),
    };
    let result = agent.run(ctx).await;
    assert!(result.success);
    assert_eq!(result.task, TaskId(7));
    assert_eq!(result.output, "stub:7 deps=1");
    // at least one event was emitted
    assert!(matches!(
        rx.try_recv(),
        Ok(HiveEvent::OutputChunk { .. }) | Ok(HiveEvent::TokenDelta { .. })
    ));
}

#[tokio::test]
async fn stub_agent_fails_for_configured_id() {
    let bus = EventBus::new(32);
    let mut ids = HashSet::new();
    ids.insert(TaskId(3));
    let agent = StubAgent { fail_ids: ids };
    let ctx = AgentContext {
        agent: AgentId(0),
        task: spec(3),
        deps: vec![],
        bus,
    };
    let result = agent.run(ctx).await;
    assert!(!result.success);
}

#[test]
fn factory_makes_agents() {
    let f = StubFactory;
    let _a = f.make(&AgentKind::Api { system: None });
    let mut ids = HashSet::new();
    ids.insert(TaskId(1));
    let ff = FailingFactory { fail_tasks: ids };
    let _b = ff.make(&AgentKind::Pty {
        command: "sh".into(),
        args: vec![],
    });
}

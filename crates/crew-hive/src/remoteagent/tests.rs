use super::*;
use crate::agent::{Agent, AgentContext, AgentFactory};
use crate::bus::{AgentId, EventBus};
use crate::graph::{AgentKind, ModelTier, TaskId, TaskSpec};
use crate::wire::RemoteReply;
use crate::worker::LoopbackTransport;
use std::sync::Arc;

fn spec(id: u64) -> TaskSpec {
    TaskSpec {
        id: TaskId(id),
        title: "t".into(),
        agent: AgentKind::Api { system: None },
        model: ModelTier::Standard,
        deps: vec![],
        prompt: "p".into(),
        specialty: String::new(),
        expertise: String::new(),
    }
}

#[tokio::test]
async fn remote_agent_dispatches_and_returns_result() {
    let tr = LoopbackTransport {
        handler: |t: crate::wire::RemoteTask| RemoteReply {
            task: t.task,
            output: "remote-ok".into(),
            success: true,
            input_tokens: 2,
            output_tokens: 2,
        },
    };
    let agent = RemoteAgent::new(Arc::new(tr));
    let bus = EventBus::new(32);
    let ctx = AgentContext {
        agent: AgentId(0),
        task: spec(5),
        deps: vec![],
        bus,
    };
    let result = agent.run(ctx).await;
    assert!(result.success);
    assert_eq!(result.output, "remote-ok");
    assert_eq!(result.task, TaskId(5));
}

#[tokio::test]
async fn remote_factory_makes_dispatching_agents() {
    // The factory shares one transport across every agent it makes.
    let transport = Arc::new(LoopbackTransport {
        handler: |t: crate::wire::RemoteTask| RemoteReply {
            task: t.task,
            output: "factory-ok".into(),
            success: true,
            input_tokens: 1,
            output_tokens: 1,
        },
    });
    let factory = RemoteFactory::new(transport);
    let agent = factory.make(&AgentKind::Api { system: None });
    let bus = EventBus::new(32);
    let ctx = AgentContext {
        agent: AgentId(1),
        task: spec(9),
        deps: vec![],
        bus,
    };
    let result = agent.run(ctx).await;
    assert!(result.success);
    assert_eq!(result.output, "factory-ok");
    assert_eq!(result.task, TaskId(9));
}

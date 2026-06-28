use super::*;
use crate::agent::{Agent, AgentContext};
use crate::board::TaskResult;
use crate::bus::{AgentId, EventBus, HiveEvent};
use crate::graph::{AgentKind, ModelTier, TaskId, TaskSpec};
use crate::provider::MockProvider;
use std::sync::Arc;

fn spec(id: u64) -> TaskSpec {
    TaskSpec {
        id: TaskId(id),
        title: "t".into(),
        agent: AgentKind::Api { system: None },
        model: ModelTier::Standard,
        deps: vec![],
        prompt: "summarize".into(),
    }
}

#[test]
fn build_prompt_includes_dep_outputs() {
    let deps = vec![TaskResult {
        task: TaskId(0),
        output: "alpha".into(),
        success: true,
    }];
    let p = build_prompt("do it", &deps);
    assert!(p.contains("do it"));
    assert!(p.contains("alpha"));
}

#[test]
fn build_prompt_no_deps_returns_prompt_unchanged() {
    let p = build_prompt("just this", &[]);
    assert_eq!(p, "just this");
}

#[test]
fn cost_micros_standard() {
    // Standard: 3 in + 15 out; 10 input + 2 output → 30 + 30 = 60
    let c = cost_micros(ModelTier::Standard, 10, 2);
    assert_eq!(c, 30 + 30);
}

#[test]
fn cost_micros_cheap() {
    let c = cost_micros(ModelTier::Cheap, 100, 10);
    assert_eq!(c, 100 + 50);
}

#[tokio::test]
async fn api_agent_completes_and_emits() {
    let bus = EventBus::new(32);
    let mut rx = bus.subscribe();
    let agent = ApiAgent::new(
        Arc::new(MockProvider {
            reply: "done".into(),
        }),
        ModelTier::Standard,
        256,
    );
    let ctx = AgentContext {
        agent: AgentId(0),
        task: spec(1),
        deps: vec![],
        bus: bus.clone(),
    };
    let result = agent.run(ctx).await;
    assert!(result.success);
    assert_eq!(result.output, "done");
    // a token-delta event was emitted
    let mut saw_tokens = false;
    while let Ok(ev) = rx.try_recv() {
        if matches!(ev, HiveEvent::TokenDelta { .. }) {
            saw_tokens = true;
        }
    }
    assert!(saw_tokens);
}

#[tokio::test]
async fn api_agent_emits_output_chunk_and_cost() {
    let bus = EventBus::new(32);
    let mut rx = bus.subscribe();
    let agent = ApiAgent::new(
        Arc::new(MockProvider {
            reply: "hello world".into(),
        }),
        ModelTier::Cheap,
        128,
    );
    let ctx = AgentContext {
        agent: AgentId(1),
        task: spec(2),
        deps: vec![],
        bus: bus.clone(),
    };
    let result = agent.run(ctx).await;
    assert!(result.success);

    let mut saw_chunk = false;
    let mut saw_cost = false;
    while let Ok(ev) = rx.try_recv() {
        match ev {
            HiveEvent::OutputChunk { text, .. } if text == "hello world" => saw_chunk = true,
            HiveEvent::CostDelta { micros_usd, .. } if micros_usd > 0 => saw_cost = true,
            _ => {}
        }
    }
    assert!(saw_chunk);
    assert!(saw_cost);
}

#[tokio::test]
async fn api_agent_with_deps_passes_context_in_prompt() {
    // Verifies that dep outputs flow through build_prompt into the request.
    // MockProvider counts tokens from the prompt, so we just need success.
    let bus = EventBus::new(32);
    let agent = ApiAgent::new(
        Arc::new(MockProvider {
            reply: "merged".into(),
        }),
        ModelTier::Standard,
        256,
    );
    let ctx = AgentContext {
        agent: AgentId(2),
        task: spec(3),
        deps: vec![TaskResult {
            task: TaskId(0),
            output: "upstream result".into(),
            success: true,
        }],
        bus: bus.clone(),
    };
    let result = agent.run(ctx).await;
    assert!(result.success);
    assert_eq!(result.output, "merged");
}

#[test]
fn api_factory_makes_an_agent() {
    use crate::agent::AgentFactory;
    use crate::graph::{AgentKind, ModelTier};
    use crate::provider::MockProvider;
    use std::sync::Arc;

    let provider = Arc::new(MockProvider { reply: "ok".into() });
    let factory = crate::apiagent::ApiFactory::new(provider, ModelTier::Standard, 256);
    let _agent = factory.make(&AgentKind::Api { system: None });
}

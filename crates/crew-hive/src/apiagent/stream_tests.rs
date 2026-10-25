//! What a task publishes while its reply streams, and what it is billed at.
use super::tests::spec;
use super::*;
use crate::agent::{Agent, AgentContext};
use crate::bus::{AgentId, EventBus, HiveEvent};
use crate::graph::ModelTier;
use crate::provider::{Completion, MockProvider, ProviderError};
use std::sync::Arc;

#[tokio::test]
async fn api_agent_streams_deltas_then_one_complete_chunk() {
    let bus = EventBus::new(64);
    let mut rx = bus.subscribe();
    let reply = "alpha beta gamma delta epsilon zeta";
    let agent = ApiAgent::new(
        Arc::new(MockProvider {
            reply: reply.into(),
        }),
        256,
    );
    let ctx = AgentContext {
        cancel: Default::default(),
        budget: crate::tools::budget::ToolBudget::solo(),
        agent: AgentId(7),
        task: spec(1),
        deps: vec![],
        bus,
    };
    let res = agent.run(ctx).await;
    assert!(res.success);

    let mut deltas: Vec<String> = Vec::new();
    let mut chunks: Vec<String> = Vec::new();
    while let Ok(ev) = rx.try_recv() {
        match ev {
            HiveEvent::OutputDelta { text, .. } => deltas.push(text),
            HiveEvent::OutputChunk { text, .. } => chunks.push(text),
            _ => {}
        }
    }
    assert!(
        deltas.len() > 1,
        "MockProvider splits into 3 groups, so the reply must arrive in pieces: {deltas:?}"
    );
    assert_eq!(
        deltas.concat(),
        reply,
        "fragments concatenate to the whole reply, losing nothing"
    );
    assert_eq!(
        chunks,
        vec![reply.to_string()],
        "exactly ONE OutputChunk, carrying the complete output"
    );
}

/// The mock's `<think>` fence reaches the bus as a `ThoughtDelta`, before
/// any reply text, and the reply's chunk is the answer alone.
#[tokio::test]
async fn api_agent_publishes_reasoning_as_thought_deltas_not_output() {
    let bus = EventBus::new(64);
    let mut rx = bus.subscribe();
    let agent = ApiAgent::new(
        Arc::new(MockProvider {
            reply: "<think>weigh it</think>alpha beta".into(),
        }),
        256,
    );
    let ctx = AgentContext {
        cancel: Default::default(),
        budget: crate::tools::budget::ToolBudget::solo(),
        agent: AgentId(7),
        task: spec(1),
        deps: vec![],
        bus,
    };
    assert!(agent.run(ctx).await.success);
    let mut order: Vec<&'static str> = Vec::new();
    let mut thoughts: Vec<String> = Vec::new();
    let mut chunks: Vec<String> = Vec::new();
    while let Ok(ev) = rx.try_recv() {
        match ev {
            HiveEvent::ThoughtDelta { text, .. } => {
                order.push("thought");
                thoughts.push(text);
            }
            HiveEvent::OutputDelta { .. } => order.push("text"),
            HiveEvent::OutputChunk { text, .. } => chunks.push(text),
            _ => {}
        }
    }
    assert_eq!(
        thoughts,
        vec!["weigh it".to_string()],
        "once, not re-published after the fact"
    );
    assert_eq!(
        order.first(),
        Some(&"thought"),
        "reasoning precedes text: {order:?}"
    );
    assert!(order[1..].iter().all(|k| *k == "text"), "{order:?}");
    assert_eq!(
        chunks,
        vec!["alpha beta".to_string()],
        "the working is not the answer"
    );
}

/// A provider that never streams still gets its reasoning to the bus — once.
#[tokio::test]
async fn a_non_streamed_thought_is_published_once_after_the_fact() {
    struct Plain;
    impl Provider for Plain {
        fn complete(
            &self,
            _req: crate::provider::CompletionRequest,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<Completion, ProviderError>> + Send>,
        > {
            Box::pin(async {
                Ok(Completion {
                    text: "answer".into(),
                    thought: "the whole working".into(),
                    input_tokens: 1,
                    output_tokens: 1,
                    ..Default::default()
                })
            })
        }
    }
    let bus = EventBus::new(64);
    let mut rx = bus.subscribe();
    let agent = ApiAgent::new(Arc::new(Plain), 256);
    let ctx = AgentContext {
        cancel: Default::default(),
        budget: crate::tools::budget::ToolBudget::solo(),
        agent: AgentId(7),
        task: spec(1),
        deps: vec![],
        bus,
    };
    assert!(agent.run(ctx).await.success);
    let mut thoughts: Vec<String> = Vec::new();
    while let Ok(ev) = rx.try_recv() {
        if let HiveEvent::ThoughtDelta { text, .. } = ev {
            thoughts.push(text);
        }
    }
    assert_eq!(thoughts, vec!["the whole working".to_string()]);
}

#[tokio::test]
async fn api_agent_bills_at_the_tasks_own_tier() {
    // Same prompt + reply (so token counts are identical), different task tier:
    // the emitted CostDelta must reflect the task's model, proving the agent
    // honours ctx.task.model rather than any fixed factory tier.
    async fn cost_for(tier: ModelTier) -> u64 {
        let bus = EventBus::new(32);
        let mut rx = bus.subscribe();
        let agent = ApiAgent::new(
            Arc::new(MockProvider {
                reply: "a b".into(),
            }),
            256,
        );
        let mut task = spec(1); // prompt "summarize" = 1 input token
        task.model = tier;
        let ctx = AgentContext {
            cancel: Default::default(),
            budget: crate::tools::budget::ToolBudget::solo(),
            agent: AgentId(0),
            task,
            deps: vec![],
            bus: bus.clone(),
        };
        let _ = agent.run(ctx).await;
        let mut cost = 0;
        while let Ok(ev) = rx.try_recv() {
            if let HiveEvent::CostDelta { micros_usd, .. } = ev {
                cost = micros_usd;
            }
        }
        cost
    }
    // 1 in, 2 out ("a b"), at each tier's own model's LIST price (`pricing`):
    // Haiku 5.5 0.1*1 + 0.5*2 = 1.1 (1 µ$); Sonnet 5.5 2*1 + 10*2 = 22;
    // Opus 5.5 4*1 + 20*2 = 44.
    assert_eq!(cost_for(ModelTier::Cheap).await, 1);
    assert_eq!(cost_for(ModelTier::Standard).await, 22);
    assert_eq!(cost_for(ModelTier::Capable).await, 44);
}

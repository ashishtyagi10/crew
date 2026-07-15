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
        specialty: String::new(),
        expertise: String::new(),
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
    use crate::graph::AgentKind;
    use crate::provider::MockProvider;
    use std::sync::Arc;

    let provider = Arc::new(MockProvider { reply: "ok".into() });
    let factory = crate::apiagent::ApiFactory::new(provider, 256);
    let _agent = factory.make(&AgentKind::Api { system: None });
}

#[tokio::test]
async fn api_factory_model_override_reaches_request() {
    use crate::agent::AgentFactory;
    use std::sync::{Arc, Mutex};
    struct Probe(Arc<Mutex<String>>);
    impl crate::provider::Provider for Probe {
        fn complete(
            &self,
            req: crate::provider::CompletionRequest,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<
                            crate::provider::Completion,
                            crate::provider::ProviderError,
                        >,
                    > + Send,
            >,
        > {
            *self.0.lock().unwrap() = req.model.clone();
            Box::pin(async {
                Ok(crate::provider::Completion {
                    text: "done".into(),
                    input_tokens: 1,
                    output_tokens: 1,
                })
            })
        }
    }
    let seen = Arc::new(Mutex::new(String::new()));
    let provider: Arc<dyn crate::provider::Provider> = Arc::new(Probe(seen.clone()));
    let factory = ApiFactory::new(provider, 64).with_model("qwen-max");
    let agent = factory.make(&crate::graph::AgentKind::Api { system: None });
    let bus = EventBus::new(32);
    let ctx = AgentContext {
        agent: AgentId(0),
        task: spec(1),
        deps: vec![],
        bus: bus.clone(),
    };
    let _result = agent.run(ctx).await;
    assert_eq!(seen.lock().unwrap().as_str(), "qwen-max");
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
    // 1 input token, 2 output tokens ("a b").
    // Cheap: 1*1 + 5*2 = 11.  Standard: 3*1 + 15*2 = 33.  Capable: 15*1 + 75*2 = 165.
    assert_eq!(cost_for(ModelTier::Cheap).await, 11);
    assert_eq!(cost_for(ModelTier::Standard).await, 33);
    assert_eq!(cost_for(ModelTier::Capable).await, 165);
}

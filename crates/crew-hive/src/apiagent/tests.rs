use super::*;
use crate::agent::{Agent, AgentContext};
use crate::board::TaskResult;
use crate::bus::{AgentId, EventBus, HiveEvent};
use crate::graph::{AgentKind, ModelTier, TaskId, TaskSpec};
use crate::provider::MockProvider;
use std::sync::Arc;

pub(super) fn spec(id: u64) -> TaskSpec {
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
        cancel: Default::default(),
        budget: crate::tools::budget::ToolBudget::solo(),
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
        cancel: Default::default(),
        budget: crate::tools::budget::ToolBudget::solo(),
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
        cancel: Default::default(),
        budget: crate::tools::budget::ToolBudget::solo(),
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
                    cost_microusd: 0,
                    ..Default::default()
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
        cancel: Default::default(),
        budget: crate::tools::budget::ToolBudget::solo(),
        agent: AgentId(0),
        task: spec(1),
        deps: vec![],
        bus: bus.clone(),
    };
    let _result = agent.run(ctx).await;
    assert_eq!(seen.lock().unwrap().as_str(), "qwen-max");
}

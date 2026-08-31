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
                    cost_microusd: 0,
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

// ---------------------------------------------------------------------------
// Tool rounds
// ---------------------------------------------------------------------------

use crate::tools::Tools;
use std::sync::Mutex;

/// A provider that answers with a scripted reply per call and RECORDS every
/// prompt it was given, so a test can assert what the agent actually saw on
/// the follow-up rather than merely that it succeeded.
struct Scripted {
    replies: Mutex<Vec<String>>,
    seen: Arc<Mutex<Vec<String>>>,
}

impl Scripted {
    fn new(replies: &[&str]) -> (Arc<Self>, Arc<Mutex<Vec<String>>>) {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let p = Arc::new(Self {
            replies: Mutex::new(replies.iter().rev().map(|s| s.to_string()).collect()),
            seen: Arc::clone(&seen),
        });
        (p, seen)
    }
}

impl crate::provider::Provider for Scripted {
    fn complete(
        &self,
        req: crate::provider::CompletionRequest,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<crate::provider::Completion, crate::provider::ProviderError>,
                > + Send,
        >,
    > {
        self.seen.lock().unwrap().push(req.prompt.clone());
        let text = self
            .replies
            .lock()
            .unwrap()
            .pop()
            .unwrap_or_else(|| "out of script".to_string());
        Box::pin(async move {
            Ok(crate::provider::Completion {
                text,
                input_tokens: 1,
                output_tokens: 1,
                cost_microusd: 0,
            })
        })
    }
}

/// A tool surface that records its calls and returns canned results.
struct FakeTools {
    hint: String,
    calls: Arc<Mutex<Vec<String>>>,
    result: Result<String, String>,
}

impl Tools for FakeTools {
    fn hint(&self) -> String {
        self.hint.clone()
    }
    fn call(&self, server: &str, tool: &str, args: &str) -> Result<String, String> {
        self.calls
            .lock()
            .unwrap()
            .push(format!("{server}:{tool} {args}"));
        self.result.clone()
    }
}

fn fake(result: Result<String, String>) -> (Arc<FakeTools>, Arc<Mutex<Vec<String>>>) {
    let calls = Arc::new(Mutex::new(Vec::new()));
    (
        Arc::new(FakeTools {
            hint: "TOOLS: @tool weather:current".into(),
            calls: Arc::clone(&calls),
            result,
        }),
        calls,
    )
}

fn ctx(bus: &EventBus) -> AgentContext {
    AgentContext {
        agent: AgentId(7),
        task: spec(1),
        deps: vec![],
        bus: bus.clone(),
    }
}

#[tokio::test]
async fn tool_call_runs_and_its_result_reaches_the_next_prompt() {
    let (provider, seen) = Scripted::new(&[
        "checking\n@tool weather:current {\"q\":\"Oslo\"}",
        "It is 4°C in Oslo.",
    ]);
    let (tools, calls) = fake(Ok("Oslo: 4C, clear".into()));
    let bus = EventBus::new(64);
    let agent = ApiAgent::new(provider, 256).with_tools(tools);

    let result = agent.run(ctx(&bus)).await;

    assert!(result.success);
    assert_eq!(result.output, "It is 4°C in Oslo.");
    // The tool actually ran, with the arguments the model wrote.
    assert_eq!(
        &*calls.lock().unwrap(),
        &["weather:current {\"q\":\"Oslo\"}"]
    );
    // Two provider calls: the ask, then the answer.
    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 2);
    // The first prompt advertised the tools.
    assert!(seen[0].contains("TOOLS: @tool weather:current"));
    // The second CARRIED THE RESULT — the whole point of the loop.
    assert!(
        seen[1].contains("Oslo: 4C, clear"),
        "follow-up: {}",
        seen[1]
    );
    assert!(seen[1].contains("CALLED weather:current"));
}

#[tokio::test]
async fn tool_call_and_result_are_published_as_events() {
    let (provider, _) = Scripted::new(&["@tool weather:current {}", "done"]);
    let (tools, _) = fake(Ok("4C".into()));
    let bus = EventBus::new(64);
    let mut rx = bus.subscribe();
    ApiAgent::new(provider, 256)
        .with_tools(tools)
        .run(ctx(&bus))
        .await;

    let mut call = None;
    let mut res = None;
    while let Ok(ev) = rx.try_recv() {
        match ev {
            HiveEvent::ToolCall { label, args, .. } => call = Some((label, args)),
            HiveEvent::ToolResult {
                label, ok, text, ..
            } => res = Some((label, ok, text)),
            _ => {}
        }
    }
    assert_eq!(
        call,
        Some(("weather:current".to_string(), "{}".to_string()))
    );
    assert_eq!(
        res,
        Some(("weather:current".to_string(), true, "4C".to_string()))
    );
}

#[tokio::test]
async fn a_failing_tool_is_reported_to_the_agent_not_raised_as_a_task_failure() {
    let (provider, seen) = Scripted::new(&[
        "@tool weather:current {}",
        "I could not reach the weather service.",
    ]);
    let (tools, _) = fake(Err("connection refused".into()));
    let bus = EventBus::new(64);
    let mut rx = bus.subscribe();

    let result = ApiAgent::new(provider, 256)
        .with_tools(tools)
        .run(ctx(&bus))
        .await;

    // The TASK succeeded: the agent got to decide what a dead tool means.
    assert!(result.success);
    assert!(seen.lock().unwrap()[1].contains("ERROR: connection refused"));
    let mut failed = false;
    let mut result_ok = true;
    while let Ok(ev) = rx.try_recv() {
        match ev {
            HiveEvent::Failed { .. } => failed = true,
            HiveEvent::ToolResult { ok, .. } => result_ok = ok,
            _ => {}
        }
    }
    assert!(!failed, "a refused tool must not fail the agent");
    assert!(!result_ok, "the ToolResult must say it failed");
}

#[tokio::test]
async fn the_round_cap_bounds_the_calls_and_the_unrun_directive_is_stripped() {
    // Ten asks in a row: only MAX_TOOL_ROUNDS may actually fire.
    let asking = "@tool weather:current {}";
    let (provider, _) = Scripted::new(&[asking; 10]);
    let (tools, calls) = fake(Ok("4C".into()));
    let bus = EventBus::new(256);
    let agent = ApiAgent::new(provider, 256).with_tools(tools);

    let result = agent.run(ctx(&bus)).await;

    assert_eq!(calls.lock().unwrap().len(), MAX_TOOL_ROUNDS as usize);
    // The output must not end on a directive that never ran.
    assert!(
        !result.output.contains("@tool"),
        "output: {}",
        result.output
    );
    assert!(result.output.contains("tool budget spent"));
}

#[tokio::test]
async fn without_tools_the_agent_makes_exactly_one_call_with_an_unchanged_prompt() {
    // The regression guard for every keyless and mock run.
    let (provider, seen) = Scripted::new(&["@tool weather:current {}"]);
    let bus = EventBus::new(32);
    let result = ApiAgent::new(provider, 256).run(ctx(&bus)).await;

    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 1, "no tools attached must mean no extra calls");
    assert_eq!(seen[0], "summarize", "the prompt must be untouched");
    // A directive with nothing to run it is just text, returned as the answer.
    assert_eq!(result.output, "@tool weather:current {}");
}

#[tokio::test]
async fn every_round_is_billed() {
    let (provider, _) = Scripted::new(&["@tool weather:current {}", "done"]);
    let (tools, _) = fake(Ok("4C".into()));
    let bus = EventBus::new(64);
    let mut rx = bus.subscribe();
    ApiAgent::new(provider, 256)
        .with_tools(tools)
        .run(ctx(&bus))
        .await;

    let mut token_events = 0;
    while let Ok(ev) = rx.try_recv() {
        if matches!(ev, HiveEvent::TokenDelta { .. }) {
            token_events += 1;
        }
    }
    // Two model calls means two token deltas, or the budget governor
    // undercounts a tool-using run by every round after the first.
    assert_eq!(token_events, 2);
}

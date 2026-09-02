use super::*;
use crate::agent::{Agent, AgentContext};
use crate::bus::{AgentId, EventBus};
use crate::graph::{AgentKind, ModelTier, TaskId, TaskSpec};
use crate::provider::{Completion, ProviderError};
use crate::tools::ToolSpec;
use crate::tools::MAX_TOOL_ROUNDS;
use std::sync::Mutex;

/// A provider that replays scripted completions and records each request, so
/// a test can assert what the model was actually shown.
struct Scripted {
    replies: Mutex<Vec<Completion>>,
    seen: Arc<Mutex<Vec<CompletionRequest>>>,
}

impl Scripted {
    fn new(replies: Vec<Completion>) -> (Arc<Self>, Arc<Mutex<Vec<CompletionRequest>>>) {
        let seen = Arc::new(Mutex::new(Vec::new()));
        (
            Arc::new(Self {
                replies: Mutex::new(replies.into_iter().rev().collect()),
                seen: Arc::clone(&seen),
            }),
            seen,
        )
    }
}

impl Provider for Scripted {
    fn supports_tools(&self) -> bool {
        true
    }
    fn complete(
        &self,
        req: CompletionRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Completion, ProviderError>> + Send>,
    > {
        self.seen.lock().unwrap().push(req);
        let c = self.replies.lock().unwrap().pop().unwrap_or_default();
        Box::pin(async move { Ok(c) })
    }
}

struct FakeTools {
    calls: Arc<Mutex<Vec<String>>>,
    result: Result<String, String>,
}

impl Tools for FakeTools {
    fn hint(&self) -> String {
        "unused on the native path".into()
    }
    fn call(&self, server: &str, tool: &str, args: &str) -> Result<String, String> {
        self.calls
            .lock()
            .unwrap()
            .push(format!("{server}:{tool} {args}"));
        self.result.clone()
    }
    fn specs(&self) -> Vec<ToolSpec> {
        vec![ToolSpec {
            server: "weather".into(),
            tool: "current".into(),
            description: "current conditions".into(),
            input_schema: serde_json::json!({"type":"object"}),
        }]
    }
}

fn calling(id: &str, name: &str) -> Completion {
    Completion {
        text: String::new(),
        input_tokens: 1,
        output_tokens: 1,
        cost_microusd: 0,
        calls: vec![ToolInvocation {
            id: id.into(),
            name: name.into(),
            input: serde_json::json!({"q": "Oslo"}),
        }],
    }
}

fn answering(text: &str) -> Completion {
    Completion {
        text: text.into(),
        input_tokens: 1,
        output_tokens: 1,
        ..Default::default()
    }
}

fn ctx(bus: &EventBus) -> AgentContext {
    AgentContext {
        budget: crate::tools::budget::ToolBudget::solo(),
        agent: AgentId(3),
        task: TaskSpec {
            id: TaskId(1),
            title: "t".into(),
            agent: AgentKind::Api { system: None },
            model: ModelTier::Standard,
            deps: vec![],
            prompt: "weather?".into(),
            specialty: String::new(),
            expertise: String::new(),
        },
        deps: vec![],
        bus: bus.clone(),
    }
}

/// Run the full agent (not `native::run` directly), so the branch that CHOOSES
/// the native path is under test too.
async fn run_agent(
    replies: Vec<Completion>,
    result: Result<String, String>,
) -> (
    crate::board::TaskResult,
    Arc<Mutex<Vec<String>>>,
    Arc<Mutex<Vec<CompletionRequest>>>,
) {
    let (provider, seen) = Scripted::new(replies);
    let calls = Arc::new(Mutex::new(Vec::new()));
    let tools = Arc::new(FakeTools {
        calls: Arc::clone(&calls),
        result,
    });
    let bus = EventBus::new(64);
    let out = crate::apiagent::ApiAgent::new(provider, 256)
        .with_tools(tools)
        .run(ctx(&bus))
        .await;
    (out, calls, seen)
}

#[tokio::test]
async fn a_structured_call_runs_and_its_result_goes_back_as_a_paired_turn() {
    let (out, calls, seen) = run_agent(
        vec![
            calling("call_1", "weather__current"),
            answering("4C in Oslo"),
        ],
        Ok("Oslo 4C clear".into()),
    )
    .await;

    assert!(out.success);
    assert_eq!(out.output, "4C in Oslo");
    assert_eq!(
        &*calls.lock().unwrap(),
        &["weather:current {\"q\":\"Oslo\"}"],
        "the wire name must resolve back to crew's server:tool"
    );

    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 2);
    // The model was shown the SCHEMA, not a text hint.
    assert_eq!(seen[0].tools.len(), 1);
    assert_eq!(seen[0].tools[0].name, "weather__current");
    assert!(
        !seen[0].prompt.contains("@tool"),
        "the text convention must not be advertised alongside: {}",
        seen[0].prompt
    );
    // The follow-up replays the call and pairs the result by id.
    assert_eq!(seen[1].turns.len(), 2);
    match (&seen[1].turns[0], &seen[1].turns[1]) {
        (Turn::Assistant { calls, .. }, Turn::ToolResults(results)) => {
            assert_eq!(calls[0].id, "call_1");
            assert_eq!(results[0].id, "call_1", "unpaired ids are a protocol error");
            assert_eq!(results[0].content, "Oslo 4C clear");
            assert!(!results[0].is_error);
        }
        other => panic!("wrong turn shape: {other:?}"),
    }
}

#[tokio::test]
async fn a_failed_tool_comes_back_flagged_as_an_error() {
    let (out, _, seen) = run_agent(
        vec![
            calling("c", "weather__current"),
            answering("could not check"),
        ],
        Err("connection refused".into()),
    )
    .await;

    assert!(out.success, "a dead tool is not a failed task");
    let seen = seen.lock().unwrap();
    match &seen[1].turns[1] {
        Turn::ToolResults(r) => {
            assert!(r[0].is_error, "without the flag a model reads this as data");
            assert!(r[0].content.contains("connection refused"));
        }
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn an_invented_tool_name_is_answered_not_executed() {
    let (out, calls, seen) = run_agent(
        vec![calling("c", "weather__forecast"), answering("sorry")],
        Ok("unused".into()),
    )
    .await;

    assert!(out.success);
    assert!(calls.lock().unwrap().is_empty(), "nothing may be executed");
    let seen = seen.lock().unwrap();
    match &seen[1].turns[1] {
        Turn::ToolResults(r) => {
            assert_eq!(r[0].id, "c", "the id must still be answered");
            assert!(r[0].is_error);
            // The model is told what it could have called.
            assert!(
                r[0].content.contains("weather__current"),
                "{}",
                r[0].content
            );
        }
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn every_call_in_a_turn_is_answered_even_past_the_per_turn_bound() {
    let many = Completion {
        text: String::new(),
        input_tokens: 1,
        output_tokens: 1,
        cost_microusd: 0,
        calls: (0..MAX_CALLS_PER_TURN + 3)
            .map(|i| ToolInvocation {
                id: format!("c{i}"),
                name: "weather__current".into(),
                input: serde_json::json!({}),
            })
            .collect(),
    };
    let (_, calls, seen) = run_agent(vec![many, answering("done")], Ok("4C".into())).await;

    assert_eq!(calls.lock().unwrap().len(), MAX_CALLS_PER_TURN);
    let seen = seen.lock().unwrap();
    match &seen[1].turns[1] {
        Turn::ToolResults(r) => {
            // A provider rejects a follow-up that leaves any tool_call id
            // unanswered, so the refused ones must still come back.
            assert_eq!(r.len(), MAX_CALLS_PER_TURN + 3);
            assert!(r[MAX_CALLS_PER_TURN].is_error);
            assert!(r[MAX_CALLS_PER_TURN].content.contains("per turn"));
        }
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn the_round_cap_stops_the_loop_and_says_so_in_the_answer() {
    let replies: Vec<Completion> = (0..10)
        .map(|i| calling(&format!("c{i}"), "weather__current"))
        .collect();
    let (out, calls, _) = run_agent(replies, Ok("4C".into())).await;

    assert_eq!(calls.lock().unwrap().len(), MAX_TOOL_ROUNDS as usize);
    assert!(out.output.contains("tool budget spent"), "{}", out.output);
}

#[tokio::test]
async fn a_provider_without_tool_support_falls_back_to_the_text_convention() {
    /// Same scripted replies, but `supports_tools()` is false.
    struct NoTools(Arc<Mutex<Vec<CompletionRequest>>>);
    impl Provider for NoTools {
        fn complete(
            &self,
            req: CompletionRequest,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<Completion, ProviderError>> + Send>,
        > {
            self.0.lock().unwrap().push(req);
            Box::pin(async move { Ok(answering("plain answer")) })
        }
    }
    let seen = Arc::new(Mutex::new(Vec::new()));
    let calls = Arc::new(Mutex::new(Vec::new()));
    let bus = EventBus::new(32);
    let out = crate::apiagent::ApiAgent::new(Arc::new(NoTools(Arc::clone(&seen))), 256)
        .with_tools(Arc::new(FakeTools {
            calls,
            result: Ok("x".into()),
        }))
        .run(ctx(&bus))
        .await;

    assert_eq!(out.output, "plain answer");
    let seen = seen.lock().unwrap();
    // No tools on the wire — and the TEXT hint is present instead, so the
    // agent is not left with no way to call anything at all.
    assert!(seen[0].tools.is_empty());
    assert!(seen[0].prompt.contains("unused on the native path"));
}

/// An invented name gets the near misses, not the whole catalog.
#[test]
fn an_unknown_tool_is_answered_with_what_was_probably_meant() {
    use crate::tools::{ToolCatalog, ToolSpec};
    let spec = |s: &str, t: &str| ToolSpec {
        server: s.into(),
        tool: t.into(),
        description: String::new(),
        input_schema: serde_json::json!({"type": "object"}),
    };
    let call = ToolInvocation {
        id: "c1".into(),
        name: "sys_run".into(),
        input: serde_json::json!({}),
    };
    let few = ToolCatalog::build(&[spec("sys", "run"), spec("sys", "read_file")]);
    let o = outcome_for_unknown(&call, &few);
    assert!(o.is_error && o.id == "c1" && o.name == "sys_run");
    assert_eq!(
        o.content,
        "unknown tool \u{201c}sys_run\u{201d} \u{2014} available: sys__read_file, sys__run"
    );
    let mut specs: Vec<ToolSpec> = (0..12).map(|i| spec("gh", &format!("issue_{i}"))).collect();
    specs.push(spec("sys", "run"));
    specs.push(spec("sys", "find_tools"));
    let many = ToolCatalog::build(&specs);
    assert_eq!(
        outcome_for_unknown(&call, &many).content,
        "unknown tool \u{201c}sys_run\u{201d} \u{2014} did you mean sys__run, sys__find_tools? (14 tools in all; sys__find_tools searches them)"
    );
}

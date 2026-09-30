//! A steer reaches a worker's next request, on the text path and the native
//! one, through the whole `ApiAgent` (the branch that picks the path included).
use super::super::ApiAgent;
use crate::agent::{Agent, AgentContext};
use crate::bus::{AgentId, EventBus};
use crate::graph::{AgentKind, ModelTier, TaskId, TaskSpec};
use crate::provider::{
    Completion, CompletionRequest, Provider, ProviderError, ToolInvocation, Turn,
};
use crate::steers::{Steers, HEAD, WORKER_TAIL};
use crate::tools::{ToolSpec, Tools};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

const SAID: &str = "also check the tests";

/// Replays `replies` and records every request; `native` says whether it
/// speaks tools, which is what picks the agent's path.
struct Scripted {
    native: bool,
    replies: Mutex<Vec<Completion>>,
    seen: Arc<Mutex<Vec<CompletionRequest>>>,
}

impl Provider for Scripted {
    fn supports_tools(&self) -> bool {
        self.native
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

struct Weather;

impl Tools for Weather {
    fn hint(&self) -> String {
        "TOOLS: @tool weather:current".into()
    }
    fn call(&self, _: &str, _: &str, _: &str) -> Result<String, String> {
        Ok("Oslo: 4C".into())
    }
    fn specs(&self) -> Vec<ToolSpec> {
        let schema = serde_json::json!({"type": "object"});
        vec![ToolSpec {
            server: "weather".into(),
            tool: "current".into(),
            description: "current conditions".into(),
            input_schema: schema,
        }]
    }
}

fn reply(text: &str, call: bool) -> Completion {
    let calls = match call {
        true => vec![ToolInvocation {
            id: "c1".into(),
            name: "weather__current".into(),
            input: serde_json::json!({}),
            bad_args: None,
        }],
        false => vec![],
    };
    Completion {
        text: text.into(),
        calls,
        input_tokens: 1,
        output_tokens: 1,
        ..Default::default()
    }
}

/// Steers whose source says [`SAID`] on its `at`th call (from 0) only.
fn said_on(at: usize) -> Steers {
    let n = AtomicUsize::new(0);
    Steers::new(move || match n.fetch_add(1, Ordering::SeqCst) == at {
        true => vec![SAID.to_string()],
        false => vec![],
    })
}

/// Run one task: a tool round, then an answer. The requests the model saw.
async fn run(native: bool, steers: Steers) -> Vec<CompletionRequest> {
    let first = match native {
        true => reply("", true),
        false => reply("checking\n@tool weather:current {}", false),
    };
    let seen = Arc::new(Mutex::new(Vec::new()));
    let replies = Mutex::new(vec![reply("4C in Oslo.", false), first]);
    let p = Arc::new(Scripted {
        native,
        replies,
        seen: Arc::clone(&seen),
    });
    let agent = ApiAgent::new(p, 256)
        .with_tools(Arc::new(Weather))
        .with_steers(steers);
    let bus = EventBus::new(64);
    let task = TaskSpec {
        id: TaskId(1),
        title: "t".into(),
        agent: AgentKind::Api { system: None },
        model: ModelTier::Standard,
        deps: vec![],
        prompt: "weather?".into(),
        specialty: String::new(),
        expertise: String::new(),
    };
    let ctx = AgentContext {
        cancel: Default::default(),
        budget: crate::tools::budget::ToolBudget::solo(),
        agent: AgentId(3),
        task,
        deps: vec![],
        bus: bus.clone(),
    };
    assert!(agent.run(ctx).await.success);
    let seen = seen.lock().unwrap().clone();
    assert_eq!(seen.len(), 2, "a tool round, then the answer");
    seen
}

#[tokio::test]
async fn a_text_worker_reads_a_steer_in_the_round_after_it_was_typed() {
    let seen = run(false, said_on(1)).await;
    assert!(!seen[0].prompt.contains(SAID), "not before it was typed");
    let p = &seen[1].prompt;
    let (head, said, tail) = (p.find(HEAD), p.find(SAID), p.find(WORKER_TAIL));
    assert!(head < said && said < tail && head.is_some(), "{p}");
    assert!(
        p.find("TOOL EXCHANGES SO FAR").unwrap() < head.unwrap(),
        "after the exchanges"
    );
}

#[tokio::test]
async fn a_native_worker_reads_a_steer_as_a_turn_after_the_results() {
    let seen = run(true, said_on(1)).await;
    assert!(seen[0].turns.is_empty());
    let turns = &seen[1].turns;
    assert!(matches!(turns[1], Turn::ToolResults(_)), "{turns:?}");
    match turns.last() {
        Some(Turn::User(t)) => assert!(t.starts_with(HEAD) && t.contains(SAID), "{t}"),
        other => panic!("the steer is the last turn: {other:?}"),
    }
}

#[tokio::test]
async fn a_worker_that_starts_after_a_steer_finds_it_in_its_first_prompt() {
    for native in [false, true] {
        let seen = run(native, said_on(0)).await;
        assert!(
            seen[0].prompt.contains(SAID),
            "native {native}: {}",
            seen[0].prompt
        );
        // Said once: not repeated as a turn of its own.
        let again = seen[1]
            .turns
            .iter()
            .any(|t| matches!(t, Turn::User(u) if u.contains(SAID)));
        assert!(!again, "native {native}");
    }
}

#[tokio::test]
async fn a_worker_with_no_steers_sends_what_it_always_sent() {
    let quiet = Steers::new(Vec::new);
    for native in [false, true] {
        let seen = run(native, quiet.clone()).await;
        assert!(
            seen.iter().all(|r| !r.prompt.contains(HEAD)),
            "native {native}"
        );
        assert!(
            seen[1].turns.iter().all(|t| !matches!(t, Turn::User(_))),
            "native {native}"
        );
    }
}

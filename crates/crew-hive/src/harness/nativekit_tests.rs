//! What the native loop's tests share: a provider that replays its replies
//! and keeps every request, a one-tool surface that keeps every call, and a
//! run of the whole agent on them.
use super::*;
use crate::agent::{Agent, AgentContext};
use crate::bus::{AgentId, EventBus};
use crate::graph::{AgentKind, ModelTier, TaskId, TaskSpec};
use crate::provider::{Completion, ProviderError};
use crate::tools::ToolSpec;
use std::sync::Mutex;

/// A provider that replays scripted completions and records each request, so
/// a test can assert what the model was actually shown.
pub(super) struct Scripted {
    replies: Mutex<Vec<Completion>>,
    seen: Arc<Mutex<Vec<CompletionRequest>>>,
}

impl Scripted {
    pub(super) fn new(replies: Vec<Completion>) -> (Arc<Self>, Arc<Mutex<Vec<CompletionRequest>>>) {
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

pub(super) struct FakeTools {
    pub(super) calls: Arc<Mutex<Vec<String>>>,
    pub(super) result: Result<String, String>,
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

pub(super) fn calling(id: &str, name: &str) -> Completion {
    Completion {
        text: String::new(),
        input_tokens: 1,
        output_tokens: 1,
        cost_microusd: 0,
        truncated: false,
        calls: vec![ToolInvocation {
            id: id.into(),
            name: name.into(),
            input: serde_json::json!({"q": "Oslo"}),
            bad_args: None,
        }],
        thought: String::new(),
        ..Default::default()
    }
}

pub(super) fn answering(text: &str) -> Completion {
    Completion {
        text: text.into(),
        input_tokens: 1,
        output_tokens: 1,
        ..Default::default()
    }
}

pub(super) fn ctx(bus: &EventBus) -> AgentContext {
    AgentContext {
        cancel: Default::default(),
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
pub(super) async fn run_agent(
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

/// One reply asking for `n` calls of the one tool, ids `c0` on.
pub(super) fn many(n: usize) -> Completion {
    Completion {
        input_tokens: 1,
        output_tokens: 1,
        calls: (0..n)
            .map(|i| ToolInvocation {
                id: format!("c{i}"),
                name: "weather__current".into(),
                input: serde_json::json!({}),
                bad_args: None,
            })
            .collect(),
        ..Default::default()
    }
}

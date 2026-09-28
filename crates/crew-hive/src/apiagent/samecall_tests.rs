//! A swarm worker does not run the same read twice in one task
//! (`tools::seen`), on either path. Executions are counted on the tool
//! surface itself.
use super::*;
use crate::bus::{AgentId, EventBus};
use crate::graph::{AgentKind, ModelTier, TaskId, TaskSpec};
use crate::provider::{Completion, ProviderError, ToolInvocation, ToolOutcome, Turn};
use crate::tools::ToolSpec;
use std::sync::Mutex;

/// Replays completions in order; keeps every request it was sent.
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

/// `fs:read` only looks; `fs:write` may change what it saw.
struct Counting(Arc<Mutex<Vec<String>>>);

impl Tools for Counting {
    fn hint(&self) -> String {
        "TOOLS: @tool fs:read {\"path\": …}".into()
    }
    fn call(&self, server: &str, tool: &str, args: &str) -> Result<String, String> {
        self.0
            .lock()
            .unwrap()
            .push(format!("{server}:{tool} {args}"));
        Ok("FILE BODY".into())
    }
    fn repeatable(&self, server: &str, tool: &str) -> bool {
        (server, tool) == ("fs", "read")
    }
    fn specs(&self) -> Vec<ToolSpec> {
        ["read", "write"]
            .map(|t| ToolSpec {
                server: "fs".into(),
                tool: t.into(),
                description: format!("{t} a file"),
                input_schema: serde_json::json!({"type": "object"}),
            })
            .to_vec()
    }
}

fn text(t: &str) -> Completion {
    Completion {
        text: t.into(),
        ..Default::default()
    }
}

fn calling(name: &str, input: serde_json::Value) -> Completion {
    Completion {
        calls: vec![ToolInvocation {
            id: format!("call-{name}"),
            name: name.into(),
            input,
        }],
        ..Default::default()
    }
}

/// Run one task on the native path when `native`, else on the `@tool` text
/// path; hand back what ran and what the provider was sent.
async fn run(native: bool, replies: Vec<Completion>) -> (Vec<String>, Vec<CompletionRequest>) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let provider = Arc::new(Scripted {
        native,
        replies: Mutex::new(replies.into_iter().rev().collect()),
        seen: Arc::clone(&seen),
    });
    let ran = Arc::new(Mutex::new(Vec::new()));
    let bus = EventBus::new(64);
    let ctx = AgentContext {
        cancel: Default::default(),
        budget: crate::tools::budget::ToolBudget::solo(),
        agent: AgentId(1),
        task: TaskSpec {
            id: TaskId(1),
            title: "t".into(),
            agent: AgentKind::Api { system: None },
            model: ModelTier::Standard,
            deps: vec![],
            prompt: "what is in a.rs?".into(),
            specialty: String::new(),
            expertise: String::new(),
        },
        deps: vec![],
        bus: bus.clone(),
    };
    ApiAgent::new(provider, 256)
        .with_tools(Arc::new(Counting(Arc::clone(&ran))))
        .run(ctx)
        .await;
    let ran = ran.lock().unwrap().clone();
    let sent = seen.lock().unwrap().drain(..).collect();
    (ran, sent)
}

/// The results the last request carried, in order.
fn results(sent: &[CompletionRequest]) -> Vec<ToolOutcome> {
    let last = sent.last().expect("a request");
    last.turns
        .iter()
        .filter_map(|t| match t {
            Turn::ToolResults(r) => Some(r.clone()),
            _ => None,
        })
        .flatten()
        .collect()
}

#[tokio::test]
async fn native_the_same_read_in_two_rounds_runs_once() {
    let read = || calling("fs__read", serde_json::json!({"path": "a.rs"}));
    let (ran, sent) = run(true, vec![read(), read(), text("done")]).await;
    assert_eq!(ran.len(), 1, "the repeat must not run: {ran:?}");
    let r = results(&sent);
    assert_eq!(r.len(), 2);
    // The first result is still in the request the pointer is read beside.
    assert_eq!(r[0].content, "FILE BODY");
    assert!(r[1].content.contains("same call as round 1"), "{:?}", r[1]);
    assert!(!r[1].is_error, "a repeat did not fail");
}

#[tokio::test]
async fn native_a_write_between_two_reads_makes_the_second_run() {
    let read = || calling("fs__read", serde_json::json!({"path": "a.rs"}));
    let write = calling("fs__write", serde_json::json!({"path": "a.rs"}));
    let (ran, _) = run(true, vec![read(), write, read(), text("done")]).await;
    assert_eq!(ran.len(), 3, "{ran:?}");
}

#[tokio::test]
async fn text_path_the_same_read_in_two_rounds_runs_once() {
    let (ran, sent) = run(
        false,
        vec![
            text("looking\n@tool fs:read {\"path\": \"a.rs\"}"),
            text("again\n@tool fs:read {\"path\":\"a.rs\"}"),
            text("done"),
        ],
    )
    .await;
    assert_eq!(ran.len(), 1, "{ran:?}");
    let last = &sent.last().unwrap().prompt;
    assert_eq!(last.matches("FILE BODY").count(), 1, "{last}");
    assert!(last.contains("same call as round 1"), "{last}");
}

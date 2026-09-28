//! A swarm worker's checklist (`sys:todo`): on the text path it heads every
//! follow-up above the exchanges, on the native path it is a user-turn note in
//! the rounds that changed it and no other, and on both an answer that left
//! steps says which. Each worker's list is its own, not the shared surface's.
use super::*;
use crate::bus::{AgentId, EventBus};
use crate::graph::{AgentKind, ModelTier, TaskId, TaskSpec};
use crate::provider::{Completion, ProviderError, ToolInvocation, Turn};
use crate::tools::todo::Checklist;
use crate::tools::ToolSpec;
use std::sync::Mutex;

/// Replays completions in order; keeps every request it was sent.
struct Scripted {
    replies: Mutex<Vec<Completion>>,
    sent: Arc<Mutex<Vec<CompletionRequest>>>,
    native: bool,
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
        self.sent.lock().unwrap().push(req);
        let c = self.replies.lock().unwrap().pop().unwrap_or_default();
        Box::pin(async move { Ok(c) })
    }
}

/// The shared surface: a list of its own (which no worker should touch) and
/// `fs:read`.
struct Shared(Checklist);

impl Tools for Shared {
    fn hint(&self) -> String {
        "TOOLS: @tool sys:todo {\"items\": …}, @tool fs:read {}".into()
    }
    fn call(&self, server: &str, tool: &str, _args: &str) -> Result<String, String> {
        match (server, tool) {
            ("sys", "todo") => unreachable!("the worker's own list answers sys:todo"),
            _ => Ok("contents".into()),
        }
    }
    fn specs(&self) -> Vec<ToolSpec> {
        let spec = |server: &str, tool: &str| ToolSpec {
            server: server.into(),
            tool: tool.into(),
            description: String::new(),
            input_schema: serde_json::json!({"type": "object"}),
        };
        vec![spec("sys", "todo"), spec("fs", "read")]
    }
    fn checklist(&self) -> Option<&Checklist> {
        Some(&self.0)
    }
}

fn list(first: &str) -> serde_json::Value {
    serde_json::json!({"items": [
        {"text": "add the command", "status": first},
        {"text": "add a test", "status": "pending"},
        {"text": "write the docs", "status": "pending"},
    ]})
}

fn text(t: String) -> Completion {
    Completion {
        text: t,
        ..Default::default()
    }
}

fn call(name: &str, input: serde_json::Value) -> Completion {
    Completion {
        calls: vec![ToolInvocation {
            id: format!("id-{name}"),
            name: name.into(),
            input,
            bad_args: None,
        }],
        ..Default::default()
    }
}

async fn run(
    replies: Vec<Completion>,
    native: bool,
) -> (Vec<CompletionRequest>, String, Arc<Shared>) {
    let sent = Arc::new(Mutex::new(Vec::new()));
    let provider = Arc::new(Scripted {
        replies: Mutex::new(replies.into_iter().rev().collect()),
        sent: Arc::clone(&sent),
        native,
    });
    let shared = Arc::new(Shared(Checklist::default()));
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
            prompt: "add a /foo command with tests and docs".into(),
            specialty: String::new(),
            expertise: String::new(),
        },
        deps: vec![],
        bus: EventBus::new(64),
    };
    let out = ApiAgent::new(provider, 256)
        .with_tools(Arc::clone(&shared) as Arc<dyn Tools>)
        .run(ctx)
        .await;
    let sent = sent.lock().unwrap().clone();
    (sent, out.output, shared)
}

const LINE: &str = "checklist: 1 of 3 done \u{2014} not done: add a test, write the docs";

#[tokio::test]
async fn the_text_path_heads_each_follow_up_with_the_workers_own_list() {
    let todo = |first: &str| text(format!("@tool sys:todo {}", list(first)));
    let replies = vec![
        todo("in_progress"),
        text("@tool fs:read {}".into()),
        todo("done"),
        text("the command is in".into()),
    ];
    let (sent, output, shared) = run(replies, false).await;
    let heads = |first: char, n: usize| {
        format!(
            "YOUR CHECKLIST:\n{first} add the command\n\u{2610} add a test\n\u{2610} write the \
             docs\n{n} of 3 done\n\nTOOL EXCHANGES SO FAR:\n"
        )
    };
    assert!(
        sent[2].prompt.contains(&heads('\u{25b6}', 0)),
        "{}",
        sent[2].prompt
    );
    assert!(
        sent[3].prompt.contains(&heads('\u{2611}', 1)),
        "{}",
        sent[3].prompt
    );
    assert_eq!(output, format!("the command is in\n\n{LINE}"));
    assert_eq!(
        shared.0.section(),
        "",
        "the shared surface's list is not the worker's"
    );
}

#[tokio::test]
async fn the_native_path_notes_the_list_only_in_the_rounds_that_changed_it() {
    let replies = vec![
        call("sys__todo", list("in_progress")),
        call("fs__read", serde_json::json!({})),
        call("sys__todo", list("done")),
        text("the command is in".into()),
    ];
    let (sent, output, _) = run(replies, true).await;
    let notes = |req: &CompletionRequest| -> Vec<String> {
        req.turns
            .iter()
            .filter_map(|t| match t {
                Turn::User(n) => Some(n.clone()),
                _ => None,
            })
            .collect()
    };
    let first = notes(&sent[1]);
    assert_eq!(first.len(), 1, "{:?}", sent[1].turns);
    assert!(first[0].starts_with("YOUR CHECKLIST:\n\u{25b6} add the command"));
    assert!(
        matches!(sent[1].turns.last(), Some(Turn::User(_))),
        "the note ends the turn"
    );
    assert_eq!(
        notes(&sent[2]),
        first,
        "a round that only read sends no new note"
    );
    let third = notes(&sent[3]);
    assert_eq!(third.len(), 2);
    assert_eq!(
        third[1],
        "YOUR CHECKLIST:\n\u{2611} add the command\n\u{2610} add a test\n\u{2610} write the \
         docs\n1 of 3 done"
    );
    assert_eq!(output, format!("the command is in\n\n{LINE}"));
}

#[tokio::test]
async fn a_worker_that_keeps_no_list_answers_as_before() {
    let replies = vec![text("@tool fs:read {}".into()), text("answer".into())];
    let (sent, output, _) = run(replies, false).await;
    assert_eq!(output, "answer");
    assert!(sent.iter().all(|r| !r.prompt.contains("YOUR CHECKLIST")));
}

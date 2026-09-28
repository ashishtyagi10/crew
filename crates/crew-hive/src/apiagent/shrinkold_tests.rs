//! A swarm worker on the `@tool` text path shortens its older results the way
//! the relay does, and runs a repeat of a shortened read again
//! (`tools::exchanges`). Executions are counted on the tool surface itself.
use super::*;
use crate::bus::{AgentId, EventBus};
use crate::graph::{AgentKind, ModelTier, TaskId, TaskSpec};
use crate::provider::{Completion, ProviderError};
use std::sync::Mutex;

/// Replays completions in order; keeps every prompt it was sent.
struct Scripted {
    replies: Mutex<Vec<Completion>>,
    prompts: Arc<Mutex<Vec<String>>>,
}

impl Provider for Scripted {
    fn complete(
        &self,
        req: CompletionRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Completion, ProviderError>> + Send>,
    > {
        self.prompts.lock().unwrap().push(req.prompt);
        let c = self.replies.lock().unwrap().pop().unwrap_or_default();
        Box::pin(async move { Ok(c) })
    }
}

/// `fs:read` answers [`file`] for its path and keeps the paths it ran.
struct Files(Arc<Mutex<Vec<String>>>);

impl Tools for Files {
    fn hint(&self) -> String {
        "TOOLS: @tool fs:read {\"path\": …}".into()
    }
    fn call(&self, _server: &str, _tool: &str, args: &str) -> Result<String, String> {
        let v: serde_json::Value = serde_json::from_str(args).map_err(|e| e.to_string())?;
        let path = v["path"].as_str().unwrap_or_default().to_string();
        self.0.lock().unwrap().push(path.clone());
        Ok(file(&path))
    }
    fn repeatable(&self, server: &str, tool: &str) -> bool {
        (server, tool) == ("fs", "read")
    }
}

/// 2,036 chars: over the 600 that are never shortened, its last line naming it.
fn file(name: &str) -> String {
    let mut lines: Vec<String> = (0..39)
        .map(|i| format!("{name} line {i:02}: {}", "y".repeat(40)))
        .collect();
    lines.push(format!("END OF {name}"));
    lines.join("\n")
}

fn read(name: &str) -> Completion {
    Completion {
        text: format!("looking\n@tool fs:read {{\"path\": \"{name}\"}}"),
        ..Default::default()
    }
}

/// Run one solo task (four tool rounds); hand back the reads that ran and
/// every prompt the provider was sent.
async fn run(replies: Vec<Completion>) -> (Vec<String>, Vec<String>) {
    let prompts = Arc::new(Mutex::new(Vec::new()));
    let provider = Arc::new(Scripted {
        replies: Mutex::new(replies.into_iter().rev().collect()),
        prompts: Arc::clone(&prompts),
    });
    let ran = Arc::new(Mutex::new(Vec::new()));
    let bus = EventBus::new(64);
    let ctx = AgentContext {
        budget: crate::tools::budget::ToolBudget::solo(),
        agent: AgentId(1),
        task: TaskSpec {
            id: TaskId(1),
            title: "t".into(),
            agent: AgentKind::Api { system: None },
            model: ModelTier::Standard,
            deps: vec![],
            prompt: "what is in these files?".into(),
            specialty: String::new(),
            expertise: String::new(),
        },
        deps: vec![],
        bus: bus.clone(),
    };
    ApiAgent::new(provider, 256)
        .with_tools(Arc::new(Files(Arc::clone(&ran))))
        .run(ctx)
        .await;
    let ran = ran.lock().unwrap().clone();
    let sent = prompts.lock().unwrap().clone();
    (ran, sent)
}

#[tokio::test]
async fn the_text_path_shortens_all_but_the_last_two_and_reruns_a_shortened_read() {
    let done = Completion {
        text: "done".into(),
        ..Default::default()
    };
    let (ran, sent) = run(vec![read("a"), read("b"), read("c"), read("a"), done]).await;
    assert_eq!(file("a").chars().count(), 2_036);
    // After three reads: `a` shortened, `b` and `c` whole.
    let third = &sent[3];
    assert!(
        third.contains("a line 00: yyyy") && !third.contains("END OF a"),
        "a kept its tail:\n{third}"
    );
    assert_eq!(
        third
            .matches(
                "\n\u{2026} (result shortened \u{2014} 2,036 chars; the call can be made \
                 again to see it all)"
            )
            .count(),
        1,
        "{third}"
    );
    assert!(third.contains(&file("b")) && third.contains(&file("c")));
    // Then `a` again: its result is shortened, so it runs rather than points.
    assert_eq!(ran, ["a", "b", "c", "a"]);
    let last = sent.last().unwrap();
    assert!(!last.contains("same call as round 1"), "{last}");
    assert!(last.contains(&file("a")), "the rerun is whole");
}

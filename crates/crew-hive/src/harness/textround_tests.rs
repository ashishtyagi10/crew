//! A text-mode worker's reply that ends with several `@tool` lines runs every
//! one of them in one round. Each test drives the whole agent with a provider
//! that has no native tools, so the `@tool` convention is the path under test.
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::agent::{Agent, AgentContext};
use crate::bus::{AgentId, EventBus};
use crate::graph::{AgentKind, ModelTier, TaskId, TaskSpec};
use crate::provider::{Completion, CompletionRequest, Provider, ProviderError};
use crate::tools::Tools;

/// Replies with `replies` in order, then an empty answer; keeps every prompt.
struct Scripted(Mutex<Vec<String>>, Arc<Mutex<Vec<String>>>);

impl Provider for Scripted {
    fn complete(
        &self,
        req: CompletionRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Completion, ProviderError>> + Send>,
    > {
        self.1.lock().unwrap().push(req.prompt);
        let text = self.0.lock().unwrap().pop().unwrap_or_default();
        Box::pin(async move {
            Ok(Completion {
                text,
                ..Default::default()
            })
        })
    }
}

/// `fs:read` only looks. Every call sleeps [`NAP`], is recorded as it starts,
/// and answers with its own arguments.
struct Sleepy(Arc<Mutex<Vec<String>>>);

const NAP: Duration = Duration::from_millis(250);

impl Tools for Sleepy {
    fn hint(&self) -> String {
        "TOOLS: @tool fs:read {\"path\": …}".into()
    }
    fn call(&self, _server: &str, tool: &str, args: &str) -> Result<String, String> {
        self.0.lock().unwrap().push(format!("{tool} {args}"));
        std::thread::sleep(NAP);
        Ok(format!("{tool} of {args}"))
    }
    fn repeatable(&self, _server: &str, tool: &str) -> bool {
        tool == "read"
    }
}

/// A reply of `said` and one `@tool fs:read` line per path.
fn reads(said: &str, paths: &[&str]) -> String {
    let calls: Vec<String> = paths
        .iter()
        .map(|p| format!("@tool fs:read {{\"path\":\"{p}\"}}"))
        .collect();
    format!("{said}\n{}", calls.join("\n"))
}

/// Run one task on the solo budget (four rounds): the calls that ran, every
/// prompt the provider was sent, the output, and how long it all took.
async fn task(replies: &[&str]) -> (Vec<String>, Vec<String>, String, Duration) {
    let prompts = Arc::new(Mutex::new(Vec::new()));
    let ran = Arc::new(Mutex::new(Vec::new()));
    let script = replies.iter().rev().map(|r| r.to_string()).collect();
    let provider = Arc::new(Scripted(Mutex::new(script), Arc::clone(&prompts)));
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
            prompt: "read them".into(),
            specialty: String::new(),
            expertise: String::new(),
        },
        deps: vec![],
        bus: EventBus::new(64),
    };
    let started = Instant::now();
    let out = crate::apiagent::ApiAgent::new(provider, 256)
        .with_tools(Arc::new(Sleepy(Arc::clone(&ran))))
        .run(ctx)
        .await;
    let took = started.elapsed();
    let ran = ran.lock().unwrap().clone();
    let prompts = prompts.lock().unwrap().clone();
    (ran, prompts, out.output, took)
}

#[tokio::test]
async fn three_reads_on_the_last_lines_run_in_one_round() {
    let first = reads("The route is split three ways.", &["a", "b", "c"]);
    let (ran, prompts, out, took) = task(&[&first, "all three read"]).await;
    assert_eq!(ran.len(), 3, "{ran:?}");
    assert_eq!(prompts.len(), 2, "one follow-up for the three");
    assert_eq!(out, "all three read");
    let follow = &prompts[1];
    for p in ["a", "b", "c"] {
        let result = format!("RESULT:\nread of {{\"path\":\"{p}\"}}");
        assert!(follow.contains(&result), "{p} missing from:\n{follow}");
    }
    assert_eq!(follow.matches("split three ways").count(), 1, "{follow}");
    assert!(
        follow.contains("You may make 1 more tool call(s)"),
        "{follow}"
    );
    // Together is one nap; in turn is three.
    assert!(took < NAP * 2, "took {took:?}");
}

/// Four rounds on the solo budget: three go to the first reply, one to the
/// second, and its other two reach the last word as refused, together.
#[tokio::test]
async fn the_calls_past_the_budget_go_to_the_last_word_together() {
    let first = reads("", &["a", "b", "c"]);
    let second = reads("then these", &["d", "e", "f"]);
    let (ran, prompts, out, _) = task(&[&first, &second, "what I found"]).await;
    assert_eq!(ran.len(), 4, "{ran:?}");
    assert_eq!(prompts.len(), 3, "first, follow-up, last word");
    let last = &prompts[2];
    assert!(last.contains("read of {\"path\":\"d\"}"), "{last}");
    assert_eq!(
        last.matches("not run \u{2014} tool budget spent").count(),
        2
    );
    assert!(last.contains("CALLED fs:read {\"path\":\"f\"}"), "{last}");
    assert!(out.starts_with("what I found"), "{out}");
}

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
        follow.contains("You may make 3 more round(s) of tool calls"),
        "one round for the three: {follow}"
    );
    // Together is one nap; in turn is three.
    assert!(took < NAP * 2, "took {took:?}");
}

/// Four rounds on the solo budget, one per reply however many calls it
/// made: five replies of three reads each run all fifteen, the fifth past the
/// budget, and the last word reads every one.
#[tokio::test]
async fn a_round_is_a_reply_and_the_batch_past_the_budget_still_runs() {
    let names = [
        "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o",
    ];
    let replies: Vec<String> = names.chunks(3).map(|p| reads("more", p)).collect();
    let mut script: Vec<&str> = replies.iter().map(String::as_str).collect();
    script.push("what I found");
    let (ran, prompts, out, _) = task(&script).await;
    assert_eq!(ran.len(), 15, "{ran:?}");
    assert_eq!(prompts.len(), 6, "first, four follow-ups, last word");
    let last = &prompts[5];
    assert!(last.contains("read of {\"path\":\"o\"}"), "{last}");
    assert!(!last.contains("not run"), "{last}");
    assert!(out.starts_with("what I found"), "{out}");
}

/// A reply past the per-reply bound runs eight, and the rest are answered
/// without running, so the model learns why.
#[tokio::test]
async fn calls_past_eight_in_one_reply_are_answered_not_run() {
    let paths: Vec<String> = (0..10).map(|i| format!("p{i}")).collect();
    let paths: Vec<&str> = paths.iter().map(String::as_str).collect();
    let first = reads("ten reads", &paths);
    let (ran, prompts, _, _) = task(&[&first, "read eight"]).await;
    assert_eq!(ran.len(), 8, "{ran:?}");
    let follow = &prompts[1];
    assert_eq!(
        follow
            .matches("not run \u{2014} at most 8 tools per turn")
            .count(),
        2,
        "{follow}"
    );
}

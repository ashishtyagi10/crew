//! The run's tool pool as the pane sees it: the aggregate `Stats` carries
//! the final `(used, total)`, and the pool running dry is said once — under
//! the lead's name, before the refused call it explains.
use super::*;
use crate::broker::testenv;
use crew_hive::StubPlanner;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

type Reply = std::pin::Pin<
    Box<
        dyn std::future::Future<Output = Result<crew_hive::Completion, crew_hive::ProviderError>>
            + Send,
    >,
>;

fn completion(text: &str) -> Reply {
    let text = text.to_string();
    Box::pin(async move {
        Ok(crew_hive::Completion {
            text,
            input_tokens: 1,
            output_tokens: 1,
            cost_microusd: 0,
            ..Default::default()
        })
    })
}

/// Asks for one tool, then answers once it has the result.
struct AskOnce;

impl crew_hive::Provider for AskOnce {
    fn complete(&self, req: crew_hive::CompletionRequest) -> Reply {
        if req.prompt.contains("TOOL EXCHANGES SO FAR") {
            completion("4C")
        } else {
            completion("@tool weather:current {}")
        }
    }
}

/// Never stops asking: the only way to drain a pool of twelve.
struct AlwaysAsks;

impl crew_hive::Provider for AlwaysAsks {
    fn complete(&self, _req: crew_hive::CompletionRequest) -> Reply {
        completion("@tool weather:current {}")
    }
}

struct Weather;

impl crew_hive::Tools for Weather {
    fn hint(&self) -> String {
        "TOOLS: @tool weather:current".into()
    }
    fn call(&self, _server: &str, _tool: &str, _args: &str) -> Result<String, String> {
        Ok("Oslo 4C clear".into())
    }
}

/// Run the stub planner's three tasks (two leaves + merge) over `provider`.
fn run(provider: Arc<dyn crew_hive::Provider>) -> Vec<PluginEvent> {
    let _env = testenv::mock("unused");
    let factory = Arc::new(crew_hive::ApiFactory::new(provider, 256).with_tools(Arc::new(Weather)));
    let mut evs = Vec::new();
    run_with(
        "what is the weather",
        Arc::new(StubPlanner { fanout: 2 }),
        factory,
        None,
        "test-model",
        Arc::new(AtomicBool::new(false)),
        None,
        &mut |ev| {
            evs.push(ev);
            Ok(())
        },
    )
    .unwrap();
    evs
}

fn stats_tools(evs: &[PluginEvent]) -> Option<(u32, u32)> {
    evs.iter().find_map(|e| match e {
        PluginEvent::Stats { agent, tools, .. } if agent.is_empty() => Some(*tools),
        _ => None,
    })?
}

fn spent_notes(evs: &[PluginEvent]) -> Vec<&str> {
    evs.iter()
        .filter_map(|e| match e {
            PluginEvent::Message { sender, text, .. }
                if sender == "agent smith" && text.starts_with("tool budget spent") =>
            {
                Some(text.as_str())
            }
            _ => None,
        })
        .collect()
}

#[test]
fn the_aggregate_stats_carries_the_pool_after_a_run_with_tools() {
    let evs = run(Arc::new(AskOnce));
    assert_eq!(
        stats_tools(&evs),
        Some((3, 12)),
        "three tasks drew one round each from four-per-task"
    );
    assert!(
        evs.iter().any(|e| matches!(
            e,
            PluginEvent::Hive {
                event: HiveEvent::ToolBudget { used: 1, total: 12 }
            }
        )),
        "the first draw crossed the wire for the live line: {evs:?}"
    );
    assert!(spent_notes(&evs).is_empty(), "nine rounds were left");
}

/// The note comes once for the run, and nothing is refused after it: the
/// batch each worker asked for past the pool still runs, for its last word.
#[test]
fn a_pool_that_runs_dry_is_said_once_and_the_batches_past_it_still_run() {
    let evs = run(Arc::new(AlwaysAsks));
    assert_eq!(stats_tools(&evs), Some((12, 12)), "every round was drawn");
    let notes = spent_notes(&evs);
    assert_eq!(
        notes,
        vec!["tool budget spent \u{2014} 12 rounds over 3 tasks; the workers answer from what they have"],
        "one note for the run, not one per agent"
    );
    let refused = evs.iter().any(|e| {
        matches!(
            e,
            PluginEvent::Hive { event: HiveEvent::ToolResult { ok: false, text, .. } }
                if text.contains("budget spent")
        )
    });
    assert!(!refused, "no call is refused for the budget: {evs:?}");
}

#[test]
fn the_note_words_its_counts() {
    assert_eq!(
        swarmtally::spent_note(4, 1),
        "tool budget spent \u{2014} 4 rounds over 1 task; the workers answer from what they have"
    );
}

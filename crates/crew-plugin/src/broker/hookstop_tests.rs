//! Stop hooks through a real `sh`, alone and driving a whole fixture turn on the session's own
//! surface: a hook that holds once costs one more model call with its reason in it, and one
//! that always holds ends at the harness's cap with a failed row. Every file is a temp file.
use std::sync::{Arc, Mutex};

use crew_hive::harness::{run, Ended, Events, TurnContext, TurnSpec};
use crew_hive::provider::{Completion, CompletionRequest, Provider, ProviderError, Turn};
use crew_hive::tools::{budget::ToolBudget, StopCheck};
use crew_hive::{AgentId, HiveEvent, ModelTier};

use super::super::tests::source;

/// Holds the first try and lets the second through, by Claude Code's flag.
const ONCE: &str = r#"grep -q '"stop_hook_active":true' && exit 0
echo '{"decision": "block", "reason": "run the tests first"}'"#;

fn stops(tag: &str, cmd: &str, on_failure: Option<&str>) -> super::super::Source {
    let mut rule = serde_json::json!({ "cmd": cmd });
    if let Some(f) = on_failure {
        rule["on_failure"] = f.into();
    }
    source(tag, &serde_json::json!({ "stop": [rule] }).to_string())
}

#[test]
fn a_stop_hook_holds_by_its_json_or_exit_2_and_reads_the_flag() {
    let once = stops("s-once", ONCE, None).load();
    let held = once.stop("done", false);
    assert_eq!(
        held,
        StopCheck::Hold(format!("run the tests first (hook `{ONCE}`)"))
    );
    assert_eq!(once.stop("done", true), StopCheck::End);
    let two = stops("s-two", "echo 'a TODO is left' >&2; exit 2", None).load();
    assert!(matches!(two.stop("x", true), StopCheck::Hold(r) if r.starts_with("a TODO is left")));
    assert_eq!(
        stops("s-one", "exit 1", None).load().stop("x", false),
        StopCheck::End
    );
}

#[test]
fn a_stop_hook_reads_the_answer() {
    let s = stops("s-reads", "cat > seen", None);
    assert_eq!(s.load().stop("all green", false), StopCheck::End);
    let seen = std::fs::read_to_string(s.cwd.as_ref().unwrap().join("seen")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&seen).unwrap();
    assert_eq!(
        (v["event"].as_str(), v["answer"].as_str()),
        (Some("stop"), Some("all green"))
    );
    assert_eq!(v["stop_hook_active"], false);
}

/// A hook that cannot run reports, and holds only when it is set to.
#[test]
fn a_stop_hook_that_cannot_run_breaks_unless_set_to_block() {
    let missing = "/no/such/stop-hook";
    let broke = stops("s-miss", missing, None).load().stop("x", false);
    assert!(matches!(broke, StopCheck::Broke(w) if w.contains("could not run")));
    let guard = stops("s-guard", missing, Some("block"))
        .load()
        .stop("x", false);
    assert!(matches!(guard, StopCheck::Hold(w) if w.contains("set to hold")));
}

/// `stop` is a known key now, and a stop rule takes no `match`.
#[test]
fn the_file_takes_stop_rules_and_refuses_a_match_on_one() {
    let bad = source("s-match", r#"{"stop": [{"match": "*", "cmd": "true"}]}"#).load();
    assert!(matches!(bad.stop("x", false), StopCheck::Broke(w) if w.contains("match")));
}

/// Answers `answer N` on its Nth call, keeping every request.
struct Answers(Arc<Mutex<Vec<CompletionRequest>>>);

impl Provider for Answers {
    fn supports_tools(&self) -> bool {
        true
    }
    fn complete(
        &self,
        req: CompletionRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Completion, ProviderError>> + Send>,
    > {
        let mut seen = self.0.lock().unwrap();
        seen.push(req);
        let text = format!("answer {}", seen.len());
        Box::pin(async move {
            Ok(Completion {
                text,
                ..Default::default()
            })
        })
    }
}

#[derive(Default)]
struct Kept(Mutex<Vec<HiveEvent>>);

impl Events for Kept {
    fn publish(&self, ev: HiveEvent) {
        self.0.lock().unwrap().push(ev);
    }
}

/// One fixture turn on the real session surface under the Stop hook `cmd`.
async fn turn(tag: &str, cmd: &str) -> (Ended, Vec<CompletionRequest>, Vec<(bool, String)>) {
    let mcp = Arc::new(Mutex::new(crate::mcp::McpHost::default()));
    let surface = super::super::super::SessionTools::for_test(mcp, true);
    let tools: Arc<dyn crew_hive::tools::Tools> =
        Arc::new(surface.with_hooks(stops(tag, cmd, None)));
    let seen = Arc::new(Mutex::new(Vec::new()));
    let events = Arc::new(Kept::default());
    let ctx = TurnContext {
        agent: AgentId(1),
        events: Arc::clone(&events) as Arc<dyn Events>,
        budget: ToolBudget::turn(4),
        cancel: Default::default(),
        advice: None,
    };
    let spec = TurnSpec {
        ask: "fix the bug".into(),
        prompt: "fix the bug".into(),
        hinted: None,
        shed: None,
        ending: None,
        call_timeout: None,
        system: None,
        model: "m".into(),
        tier: ModelTier::Standard,
        max_tokens: 64,
    };
    let provider = Arc::new(Answers(Arc::clone(&seen)));
    let ended = run(ctx, spec, provider, Some(tools), None).await;
    let rows = events.0.lock().unwrap().clone().into_iter();
    let rows = rows.filter_map(|e| match e {
        HiveEvent::ToolResult {
            label, ok, text, ..
        } if label == "hook:stop" => Some((ok, text)),
        _ => None,
    });
    let requests = seen.lock().unwrap().clone();
    (ended, requests, rows.collect())
}

#[tokio::test]
async fn a_hook_that_holds_once_costs_one_more_call_with_its_reason() {
    let (ended, requests, rows) = turn("s-turn-once", ONCE).await;
    assert_eq!(requests.len(), 2);
    assert_eq!(ended, Ended::Answered("answer 2".into()));
    let note = requests[1]
        .turns
        .iter()
        .any(|t| matches!(t, Turn::User(n) if n.contains("run the tests first")));
    assert!(note, "{:?}", requests[1].turns);
    assert_eq!(rows.len(), 1);
    assert!(
        rows[0].0
            && rows[0]
                .1
                .starts_with("held the turn (1 of 3): run the tests first")
    );
}

#[tokio::test]
async fn a_hook_that_always_holds_ends_at_the_cap_with_a_failed_row() {
    let always = r#"echo '{"decision": "block", "reason": "never done"}'"#;
    let (ended, requests, rows) = turn("s-turn-always", always).await;
    assert_eq!(requests.len(), 4, "the first answer and three holds");
    assert_eq!(ended, Ended::Answered("answer 4".into()));
    let (ok, last) = rows.last().unwrap();
    assert!(
        !ok && last.contains("held 3 times already") && last.contains("never done"),
        "{last}"
    );
}

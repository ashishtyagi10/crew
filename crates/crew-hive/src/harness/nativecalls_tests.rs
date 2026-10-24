//! Calls the native loop answers without running them: a name the model
//! invented, and the calls of one reply past the per-turn bound.
use super::kit::*;
use super::*;
use crate::agent::Agent;
use crate::bus::EventBus;
use std::sync::Mutex;

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
    let replies = vec![many(MAX_CALLS_PER_TURN + 3), answering("done")];
    let (_, calls, seen) = run_agent(replies, Ok("4C".into())).await;

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
        bad_args: None,
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

/// The refusal past the per-turn bound reaches the BUS too, one `ToolResult
/// { ok: false }` per dropped call: the pane draws the block from these
/// events, so a call answered only on the wire back to the model was a call
/// the person watching never saw refused.
#[tokio::test]
async fn calls_refused_past_the_per_turn_bound_publish_a_failed_result_each() {
    let (provider, _) = Scripted::new(vec![many(MAX_CALLS_PER_TURN + 2), answering("done")]);
    let tools = Arc::new(FakeTools {
        calls: Arc::new(Mutex::new(Vec::new())),
        result: Ok("4C".into()),
    });
    let bus = EventBus::new(256);
    let mut rx = bus.subscribe();
    crate::apiagent::ApiAgent::new(provider, 256)
        .with_tools(tools)
        .run(ctx(&bus))
        .await;
    let mut refused = Vec::new();
    while let Ok(ev) = rx.try_recv() {
        if let HiveEvent::ToolResult {
            ok: false,
            text,
            label,
            ms,
            ..
        } = ev
        {
            refused.push((label, text, ms));
        }
    }
    assert_eq!(
        refused.len(),
        2,
        "one refusal per dropped call: {refused:?}"
    );
    assert_eq!(refused[0].0, "weather:current");
    assert_eq!(
        refused[0].1,
        format!("not run \u{2014} at most {MAX_CALLS_PER_TURN} tools per turn")
    );
    assert_eq!(refused[0].2, 0, "nothing ran, so no duration");
}

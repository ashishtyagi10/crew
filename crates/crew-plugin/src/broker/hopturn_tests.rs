//! A relay hop of an API agent with tools, run on the harness: driven whole
//! through `Broker::run`, a scripted model behind a real `ApiAdapter`. What
//! the model is shown, what the pane is sent, and what the thread is billed.
use crew_hive::HiveEvent;

use super::kit::*;
use crate::broker::hop::HopKind;
use crate::PluginEvent;

/// A provider that speaks tool-use gets the tools on the wire, each with its
/// schema, and no `@tool` hint beside them; the result goes back as a turn,
/// and the answer routes as the hop's.
#[test]
fn a_native_provider_gets_its_tools_on_the_wire() {
    let t = go(
        true,
        vec![
            Ok(calling("fs__read", serde_json::json!({"path": "a.rs"}))),
            Ok(said("a.rs holds main\n@done")),
        ],
    );
    assert_eq!(t.ran, vec!["fs:read {\"path\":\"a.rs\"}"]);
    assert_eq!(t.seen.len(), 2);
    assert!(!t.seen[0].tools.is_empty(), "tools on the wire");
    assert!(!t.seen[0].prompt.contains("TOOLS:"), "{}", t.seen[0].prompt);
    assert!(t.seen[0].prompt.contains("HOW TO REPLY"), "the frame");
    assert_eq!(t.seen[1].turns.len(), 2, "the call, then its result");
    assert_eq!(t.done(), Some("a.rs holds main"));
    assert!(t.errors().is_empty(), "{:?}", t.errors());
}

/// Without tool-use the hint sits where the relay's frame puts it, inside
/// TASK and above HOW TO REPLY, and every follow-up re-states it.
#[test]
fn a_text_provider_reads_the_hint_inside_the_task() {
    let t = go(
        false,
        vec![
            Ok(said("looking\n@tool fs:read {\"path\": \"a.rs\"}")),
            Ok(said("a.rs holds main\n@done")),
        ],
    );
    assert_eq!(t.ran.len(), 1, "{:?}", t.ran);
    for p in t.seen.iter().map(|r| &r.prompt) {
        let (hint, how) = (p.find("TOOLS:").unwrap(), p.find("HOW TO REPLY").unwrap());
        assert!(hint < how, "{p}");
        assert_eq!(p.matches("TOOLS:").count(), 1, "{p}");
    }
    let follow = &t.seen[1].prompt;
    assert!(follow.contains("RESULT:\ncontents of"), "{follow}");
    assert!(follow.contains("You may make 3 more round(s)"), "{follow}");
    assert_eq!(t.done(), Some("a.rs holds main"));
}

/// The pane sees the turn the way it sees Claude Code's: each call and its
/// result on the tool lane under the agent's name, the reply streamed, its
/// tokens ticked; one Dialing hop for the hop, not one a round.
#[test]
fn the_pane_gets_the_tool_lane_and_the_stream() {
    let t = go(
        true,
        vec![
            Ok(calling("fs__read", serde_json::json!({"path": "a.rs"}))),
            Ok(said("a.rs holds main\n@done")),
        ],
    );
    let lane = t.tool_events();
    assert!(matches!(lane[0], HiveEvent::ToolCall { label, .. } if label == "fs:read"));
    assert!(matches!(lane[1], HiveEvent::ToolResult { label, ok: true, .. } if label == "fs:read"));
    let dials = t.hops.iter().filter(|h| h.kind == HopKind::Dialing).count();
    assert_eq!(dials, 1);
    let ticked = t
        .events
        .iter()
        .any(|e| matches!(e, PluginEvent::StatsTick { .. }));
    assert!(ticked, "{:?}", t.events);
}

/// An accepted checklist goes out as the plan and makes no tool line; a
/// refused one changed nothing, so nothing goes out for it.
#[test]
fn a_checklist_write_goes_out_as_the_plan_only_when_accepted() {
    let todo =
        |items: serde_json::Value| calling("sys__todo", serde_json::json!({ "items": items }));
    let step = serde_json::json!([{"text": "read a.rs", "status": "pending"}]);
    let t = go(true, vec![Ok(todo(step)), Ok(said("planned\n@done"))]);
    let plans = t
        .events
        .iter()
        .filter(|e| matches!(e, PluginEvent::Todo { .. }))
        .count();
    assert_eq!(plans, 1, "{:?}", t.events);
    assert!(
        t.tool_events().is_empty(),
        "no tool line: {:?}",
        t.tool_events()
    );
    let t = go(
        true,
        vec![Ok(todo(serde_json::json!([]))), Ok(said("ok\n@done"))],
    );
    let plans = t
        .events
        .iter()
        .filter(|e| matches!(e, PluginEvent::Todo { .. }))
        .count();
    assert_eq!(plans, 0, "{:?}", t.events);
}

/// Every model call of the turn is one exchange, billed with the usage it
/// reported, and the tool result's size joins the estimate.
#[test]
fn every_call_of_the_turn_is_in_the_stats() {
    let t = go(
        true,
        vec![
            Ok(calling("fs__read", serde_json::json!({"path": "a.rs"}))),
            Ok(said("done\n@done")),
        ],
    );
    assert_eq!(t.stats.exchanges, 2);
    assert_eq!(t.stats.tok_in, 200);
    assert_eq!(t.stats.tok_out, 20);
    let done = t.hops.iter().find(|h| h.kind == HopKind::Done).unwrap();
    assert_eq!(
        done.usage.input_tokens, 100,
        "the latest call's context fill"
    );
}

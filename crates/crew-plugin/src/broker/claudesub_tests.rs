//! The lines are the shapes `claude` 2.1.318 printed on 2026-11-05 for a
//! lead that spawned a foreground `Agent` (ids trimmed, inputs cut short).
use std::sync::{Arc, Mutex};

use crew_hive::claudestream::parse_line;
use crew_hive::HiveEvent;

use super::super::adapter::HopStream;
use super::super::claudelane::Lane;

const AGENT: &str = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"toolu_A","name":"Agent","input":{"description":"explore the renderer","subagent_type":"general-purpose","prompt":"read a.rs"}}]},"parent_tool_use_id":null}"#;
const SUB_PROMPT: &str = r#"{"type":"user","message":{"role":"user","content":[{"type":"text","text":"read a.rs"}]},"parent_tool_use_id":"toolu_A"}"#;

fn sub_use(id: &str, name: &str) -> String {
    format!(
        r#"{{"type":"assistant","message":{{"content":[{{"type":"tool_use","id":"{id}","name":"{name}","input":{{"file_path":"a.rs"}}}}]}},"parent_tool_use_id":"toolu_A"}}"#
    )
}

fn sub_result(id: &str, err: bool) -> String {
    format!(
        r#"{{"type":"user","message":{{"content":[{{"type":"tool_result","tool_use_id":"{id}","content":"x","is_error":{err}}}]}},"parent_tool_use_id":"toolu_A"}}"#
    )
}

const AGENT_DONE: &str = r#"{"type":"user","message":{"content":[{"tool_use_id":"toolu_A","type":"tool_result","content":[{"type":"text","text":"a.rs holds fn a"}]}]},"parent_tool_use_id":null}"#;

/// Run `lines` through a lane; the hive events the pane was handed.
fn drive(lines: &[String]) -> Vec<HiveEvent> {
    let got = Arc::new(Mutex::new(Vec::new()));
    let sink = got.clone();
    let stream = HopStream {
        on_tokens: Arc::new(|_| {}),
        on_text: Arc::new(|_| {}),
        on_thought: Arc::new(|_| {}),
        on_tool: Arc::new(move |e| sink.lock().unwrap().push(e)),
    };
    let mut lane = Lane::new();
    for l in lines {
        lane.forward(parse_line(l).as_ref(), &stream);
    }
    let out = got.lock().unwrap().clone();
    out
}

#[test]
fn a_subagents_calls_fold_into_its_agent_line() {
    let mut lines = vec![AGENT.to_string(), SUB_PROMPT.to_string()];
    for (id, name, err) in [
        ("r1", "Read", false),
        ("g1", "Grep", true),
        ("r2", "Read", false),
        ("t1", "TodoWrite", false),
    ] {
        lines.push(sub_use(id, name));
        lines.push(sub_result(id, err));
    }
    lines.push(AGENT_DONE.to_string());
    let evs = drive(&lines);
    // One call and one result reach the pane: the lead's own Agent call.
    // The subagent's TodoWrite never does, so the lead's plan stands.
    assert_eq!(evs.len(), 2, "{evs:?}");
    assert!(matches!(&evs[0], HiveEvent::ToolCall { label, .. } if label == "Agent"));
    let HiveEvent::ToolResult {
        label, text, ok, ..
    } = &evs[1]
    else {
        panic!("{evs:?}")
    };
    assert_eq!((label.as_str(), *ok), ("Agent", true));
    assert_eq!(
        text,
        "4 calls \u{00b7} 2 Read \u{00b7} 1 Grep \u{00b7} 1 TodoWrite \u{00b7} 1 failed\na.rs holds fn a"
    );
}

#[test]
fn a_call_with_no_subagent_work_keeps_its_result() {
    let evs = drive(&[AGENT.to_string(), AGENT_DONE.to_string()]);
    let HiveEvent::ToolResult { text, .. } = &evs[1] else {
        panic!("{evs:?}")
    };
    assert_eq!(text, "a.rs holds fn a");
}

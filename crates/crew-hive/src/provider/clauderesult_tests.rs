use serde_json::json;

use super::{denials, note, Denied};
use crate::claudestream::{parse_line, StreamEvent};

/// The shape a real `claude -p` printed on 2026-11-02 in default mode, asked
/// to write a file and run a command it was not allowed to.
fn captured() -> serde_json::Value {
    json!({"type":"result","subtype":"success","is_error":false,"result":"needed approval",
    "permission_denials":[
      {"tool_name":"Write","tool_use_id":"t1",
       "tool_input":{"file_path":"/x/hello.txt","content":"hi"}},
      {"tool_name":"Bash","tool_use_id":"t2",
       "tool_input":{"command":"touch made.txt","description":"Create made.txt"}},
      {"tool_name":"Bash","tool_use_id":"t3",
       "tool_input":{"command":"touch made.txt","description":"again"}}]})
}

fn d(tool: &str, on: &str) -> Denied {
    Denied {
        tool: tool.into(),
        on: on.into(),
    }
}

#[test]
fn the_result_line_carries_each_refusal_once() {
    let line = captured().to_string();
    let Some(StreamEvent::Result(r)) = parse_line(&line) else {
        panic!("a result line");
    };
    assert_eq!(
        r.denied,
        vec![d("Write", "/x/hello.txt"), d("Bash", "`touch made.txt`")]
    );
    let none = json!({"type":"result","subtype":"success","permission_denials":[]});
    assert!(denials(&none).is_empty());
    assert!(denials(&json!({"type":"result"})).is_empty());
}

#[test]
fn the_note_names_what_was_refused_and_where_to_allow_it() {
    assert_eq!(note(&[]), None);
    let said = note(&denials(&captured())).unwrap();
    assert!(
        said.starts_with(
            "Claude Code was not allowed to: Write /x/hello.txt \u{b7} Bash `touch made.txt`."
        ),
        "{said}"
    );
    assert!(said.contains("permissions.allow"), "{said}");
    let many: Vec<Denied> = (0..6).map(|i| d("Read", &format!("f{i}"))).collect();
    let said = note(&many).unwrap();
    assert!(said.contains("Read f3 (+2 more)."), "{said}");
    assert!(!said.contains("f4"), "{said}");
}

#[test]
fn a_long_command_is_clipped_and_a_bare_tool_is_named_alone() {
    let long = "x".repeat(200);
    let v = json!({"permission_denials":[
        {"tool_name":"Bash","tool_input":{"command": long}},
        {"tool_name":"WebSearch","tool_input":{"query":"q"}}]});
    let got = denials(&v);
    assert_eq!(got[0].on.chars().count(), 62, "{}", got[0].on);
    assert!(got[0].on.ends_with("\u{2026}`"));
    assert_eq!(got[1], d("WebSearch", ""));
    assert!(note(&got).unwrap().contains(" \u{b7} WebSearch."));
}

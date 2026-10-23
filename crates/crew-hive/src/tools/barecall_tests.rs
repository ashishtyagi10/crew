use super::bare_call;
use crate::tools::{split_tool_calls, stray_calls, ToolCall};

fn read(path: &str) -> ToolCall {
    ToolCall {
        server: "sys".into(),
        tool: "read_file".into(),
        args: format!("{{\"path\": \"{path}\"}}"),
    }
}

/// qwen-max's reply on the live probe (2026-10-22): its next reads written
/// without `@tool`, fenced as `sh`, one short line under them. They never
/// ran, and the turn ended on them; now they are the calls they meant.
#[test]
fn bare_calls_in_a_fence_are_calls() {
    let reply = "We need to update the test files.\n\n```sh\n\
                 sys:read_file {\"path\": \"a.rs\"}\n\
                 sys:read_file {\"path\": \"b.rs\"}\n```\n\n\
                 Once I have them, I will make the changes.";
    let (said, calls) = split_tool_calls(reply).expect("calls");
    assert_eq!(said, "We need to update the test files.");
    assert_eq!(calls, vec![read("a.rs"), read("b.rs")]);
}

/// Only a name and JSON make one: a log line, a URL, a time, prose with a
/// colon, a name with no JSON after it, are words.
#[test]
fn only_a_tool_name_with_json_is_a_bare_call() {
    assert!(bare_call("sys:read_file {\"path\": \"a.rs\"}"));
    assert!(bare_call("github:list-issues {}"));
    for line in [
        "error[E0308]: mismatched types {",
        "warning: unused {x}",
        "https://x.org/a {",
        "Note:this {",
        "sys:read_file a.rs",
        "at 10:30 {",
        "sys:edit {\"path\": \"a.rs\"",
    ] {
        assert!(!bare_call(line), "{line}");
    }
    assert_eq!(
        stray_calls("x\nsys:git {\"args\": \"log\"}\ny"),
        ["sys:git"]
    );
}

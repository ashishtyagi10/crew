use super::*;

#[test]
fn parse_reads_the_last_non_empty_line() {
    let c = parse_tool_call("thinking out loud\n@tool fs:read {\"path\": \"x\"}\n\n").unwrap();
    assert_eq!(c.server, "fs");
    assert_eq!(c.tool, "read");
    assert_eq!(c.args, "{\"path\": \"x\"}");
    assert_eq!(c.label(), "fs:read");
}

#[test]
fn parse_tolerates_markdown_wrappers_and_case() {
    let c = parse_tool_call("done soon\n**@Tool `fs:read` {}**").unwrap();
    assert_eq!(c.label(), "fs:read");
}

#[test]
fn parse_rejects_non_directives() {
    assert!(parse_tool_call("just an answer").is_none());
    assert!(parse_tool_call("@tool malformed-no-colon {}").is_none());
    assert!(parse_tool_call("").is_none());
    // A directive that is not LAST is not a call: an agent quoting the syntax
    // while explaining itself must not fire a tool.
    assert!(parse_tool_call("@tool fs:read {}\nactually, never mind").is_none());
}

#[test]
fn parse_allows_missing_arguments() {
    let c = parse_tool_call("@tool sys:list_dir").unwrap();
    assert_eq!(c.label(), "sys:list_dir");
    assert_eq!(c.args, "");
}

#[test]
fn augment_with_an_empty_hint_is_byte_identical() {
    // The no-tools swarm must be unchanged by this module's existence.
    assert_eq!(augment("do the thing", ""), "do the thing");
}

#[test]
fn augment_appends_the_hint_after_a_blank_line() {
    assert_eq!(augment("body", "TOOLS: …"), "body\n\nTOOLS: …");
}

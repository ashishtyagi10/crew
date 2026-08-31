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

// ---------------------------------------------------------------------------
// Wire names
// ---------------------------------------------------------------------------

fn spec(server: &str, tool: &str) -> ToolSpec {
    ToolSpec {
        server: server.into(),
        tool: tool.into(),
        description: "d".into(),
        input_schema: serde_json::json!({"type": "object"}),
    }
}

#[test]
fn a_wire_name_is_legal_and_resolves_back() {
    let c = ToolCatalog::build(&[spec("weather", "current")]);
    assert_eq!(c.names(), vec!["weather__current"]);
    assert_eq!(c.resolve("weather__current"), Some(("weather", "current")));
}

#[test]
fn illegal_characters_are_replaced_not_passed_through() {
    // A dotted server name is ordinary in mcp.json and illegal on the wire.
    let c = ToolCatalog::build(&[spec("my.server:v2", "get/thing")]);
    let name = c.names()[0].to_string();
    assert!(
        name.chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'),
        "{name}"
    );
    // And it still points at the ORIGINAL, unsanitised identity.
    assert_eq!(c.resolve(&name), Some(("my.server:v2", "get/thing")));
}

#[test]
fn names_that_sanitize_alike_do_not_collide() {
    // `a.b` and `a:b` both sanitise to `a_b`; resolving either to the other's
    // server would call the wrong machine.
    let c = ToolCatalog::build(&[spec("a.b", "run"), spec("a:b", "run")]);
    let names = c.names();
    assert_eq!(names.len(), 2);
    assert_ne!(names[0], names[1], "collision: {names:?}");
    assert_eq!(c.resolve(names[0]), Some(("a.b", "run")));
    assert_eq!(c.resolve(names[1]), Some(("a:b", "run")));
}

#[test]
fn long_names_are_truncated_to_the_provider_limit_and_stay_unique() {
    let long = "x".repeat(90);
    let c = ToolCatalog::build(&[spec(&long, "one"), spec(&long, "two")]);
    for n in c.names() {
        assert!(n.len() <= 64, "{} chars: {n}", n.len());
    }
    let names = c.names();
    assert_ne!(names[0], names[1]);
    assert_eq!(c.resolve(names[1]), Some((long.as_str(), "two")));
}

#[test]
fn an_invented_name_resolves_to_nothing() {
    let c = ToolCatalog::build(&[spec("weather", "current")]);
    assert_eq!(c.resolve("weather_forecast"), None);
}

//! Each tagged shape here is one a model writes from its training when the
//! prompt asked for `@tool`; before `untag` the raw block was the answer.
use super::*;
use crate::tools::{parse_tool_call, split_tool_call, split_tool_calls, ToolCall};

fn call(server: &str, tool: &str, args: &str) -> Option<ToolCall> {
    Some(ToolCall {
        server: server.into(),
        tool: tool.into(),
        args: args.into(),
    })
}

#[test]
fn the_probes_qwen_block_is_a_call_with_its_arguments_as_written() {
    // qwen3.8-max, 2026-09-30: `path` before `offset`, which sorting would swap.
    let args = r#"{"path": ".crew/out/run-20260930-003922-1.txt", "offset": 0}"#;
    let reply = format!(
        "<tool_call>\n{{\"name\": \"sys:read_file\", \"arguments\": {args}}}\n</tool_call>"
    );
    assert_eq!(parse_tool_call(&reply), call("sys", "read_file", args));
}

#[test]
fn what_the_model_said_before_the_block_is_kept_and_the_block_is_not() {
    let block = r#"<tool_call>{"name": "sys:grep", "arguments": {"pattern": "CAP"}}</tool_call>"#;
    let (said, c) = split_tool_call(&format!("The cap is set somewhere.\n{block}")).unwrap();
    assert_eq!(
        (said.as_str(), c.tool.as_str()),
        ("The cap is set somewhere.", "grep")
    );
    let (said, _) = split_tool_call(&format!("Reading it now. {block}")).unwrap();
    assert_eq!(said, "Reading it now.");
}

#[test]
fn a_block_whose_closing_tag_never_came_is_still_a_call() {
    let reply = "<tool_call>\n{\"name\": \"sys:list_dir\", \"arguments\": {\"path\": \"src\"}}";
    assert_eq!(
        parse_tool_call(reply),
        call("sys", "list_dir", "{\"path\": \"src\"}")
    );
}

#[test]
fn arguments_sent_as_a_json_string_are_that_json() {
    let reply = r#"<tool_call>{"name": "sys:grep", "arguments": "{\"pattern\": \"TASK_CAP\"}"}</tool_call>"#;
    assert_eq!(
        parse_tool_call(reply),
        call("sys", "grep", "{\"pattern\": \"TASK_CAP\"}")
    );
}

#[test]
fn input_and_parameters_carry_arguments_too_and_none_is_empty() {
    let input = r#"<tool_call>{"name": "sys:glob", "input": {"pattern": "*.md"}}</tool_call>"#;
    assert_eq!(
        parse_tool_call(input),
        call("sys", "glob", r#"{"pattern": "*.md"}"#)
    );
    let params =
        r#"<tool_call>{"name": "sys:glob", "parameters": {"pattern": "*.rs"}}</tool_call>"#;
    assert_eq!(
        parse_tool_call(params),
        call("sys", "glob", r#"{"pattern": "*.rs"}"#)
    );
    let bare = r#"<tool_call>{"name": "sys:list_dir"}</tool_call>"#;
    assert_eq!(parse_tool_call(bare), call("sys", "list_dir", "{}"));
}

#[test]
fn a_nested_key_of_the_same_name_does_not_stand_in_for_the_arguments() {
    let reply = r#"<tool_call>{"input": {"arguments": 5}, "name": "sys:x", "arguments": {"z": 1, "a": 2}}</tool_call>"#;
    assert_eq!(
        parse_tool_call(reply),
        call("sys", "x", r#"{"z": 1, "a": 2}"#)
    );
}

#[test]
fn consecutive_blocks_are_every_call_in_the_order_written() {
    let reply = "Both files.\n<tool_call>\n{\"name\": \"sys:read_file\", \"arguments\": {\"path\": \"a.rs\"}}\n</tool_call>\n<tool_call>\n{\"name\": \"sys:read_file\", \"arguments\": {\"path\": \"b.rs\"}}\n</tool_call>";
    let (said, calls) = split_tool_calls(reply).unwrap();
    let paths: Vec<&str> = calls.iter().map(|c| c.args.as_str()).collect();
    assert_eq!(said, "Both files.");
    assert_eq!(paths, [r#"{"path": "a.rs"}"#, r#"{"path": "b.rs"}"#]);
}

#[test]
fn a_block_with_a_paragraph_under_it_is_an_example_not_a_call() {
    let reply = "Qwen calls a tool like this:\n<tool_call>{\"name\": \"sys:grep\", \"arguments\": {}}</tool_call>\nThat is the Hermes shape.\nMany open models write it.\nCrew reads it as a call when it ends the reply.";
    assert_eq!(parse_tool_call(reply), None);
}

#[test]
fn a_block_that_names_no_crew_tool_or_never_closes_stays_as_written() {
    let bare = r#"<tool_call>{"name": "read_file", "arguments": {"path": "a"}}</tool_call>"#;
    let cut = "<tool_call>\n{\"name\": \"sys:read_file\", \"arguments\": {\"path\": \"a";
    for reply in [bare, cut] {
        assert_eq!(untag(reply), reply);
        assert_eq!(parse_tool_call(reply), None);
    }
    assert!(matches!(untag("no tags at all"), Cow::Borrowed(_)));
}

#[test]
fn qwen_coders_xml_block_is_a_call_with_strings_kept_strings() {
    let reply = "<tool_call>\n<function=sys:grep>\n<parameter=pattern>\n404\n</parameter>\n<parameter=ignore_case>\ntrue\n</parameter>\n</function>\n</tool_call>";
    assert_eq!(
        parse_tool_call(reply),
        call("sys", "grep", r#"{"pattern": "404", "ignore_case": true}"#)
    );
}

#[test]
fn an_xml_parameter_spanning_lines_keeps_its_inner_newlines() {
    let reply = "<tool_call>\n<function=sys:edit>\n<parameter=path>\na.rs\n</parameter>\n<parameter=new>\nfn a() {\n}\n</parameter>\n</function>\n</tool_call>";
    let c = parse_tool_call(reply).unwrap();
    let v: Value = serde_json::from_str(&c.args).unwrap();
    assert_eq!(
        (v["path"].as_str(), v["new"].as_str()),
        (Some("a.rs"), Some("fn a() {\n}"))
    );
}

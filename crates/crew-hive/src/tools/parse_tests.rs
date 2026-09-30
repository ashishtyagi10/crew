//! Each shape here is one a real model wrote and the last-line parser read as
//! the answer, showing the raw `@tool` text and never running the tool.
use super::*;

fn call(server: &str, tool: &str, args: &str) -> Option<ToolCall> {
    Some(ToolCall {
        server: server.into(),
        tool: tool.into(),
        args: args.into(),
    })
}

#[test]
fn a_call_fenced_in_a_code_block_is_a_call() {
    let reply = "Let me check.\n```\n@tool sys:read_file {\"path\": \"src/main.rs\"}\n```";
    assert_eq!(
        parse_tool_call(reply),
        call("sys", "read_file", "{\"path\": \"src/main.rs\"}")
    );
}

#[test]
fn json_spread_over_lines_is_read_to_its_close_as_written() {
    let json = "{\n  \"path\": \"a.rs\",\n  \"old\": \"x\",\n  \"new\": \"y\"\n}";
    let reply = format!("@tool sys:edit {json}");
    assert_eq!(parse_tool_call(&reply), call("sys", "edit", json));
}

#[test]
fn a_short_sentence_after_the_call_does_not_hide_it() {
    let reply = "@tool sys:grep {\"pattern\": \"TASK_CAP\"}\nI'll read the results next.";
    assert_eq!(
        parse_tool_call(reply),
        call("sys", "grep", "{\"pattern\": \"TASK_CAP\"}")
    );
}

#[test]
fn a_routing_line_after_the_call_does_not_hide_it() {
    let reply = "@tool sys:glob {\"pattern\": \"**/*.rs\"}\n@done";
    assert_eq!(
        parse_tool_call(reply),
        call("sys", "glob", "{\"pattern\": \"**/*.rs\"}")
    );
    let reply = "@tool sys:glob {\"pattern\": \"*.md\"}\n**@next coder**";
    assert_eq!(
        parse_tool_call(reply),
        call("sys", "glob", "{\"pattern\": \"*.md\"}")
    );
}

#[test]
fn json_under_a_bare_call_in_a_json_fence_is_its_args() {
    let json = "{\n  \"path\": \"src/lib.rs\",\n  \"offset\": 0\n}";
    let reply = format!("Checking the file.\n```json\n@tool sys:read_file\n{json}\n```");
    assert_eq!(parse_tool_call(&reply), call("sys", "read_file", json));
}

#[test]
fn braces_and_quotes_inside_strings_do_not_close_the_json() {
    let json = "{\"old\": \"fn a() {\",\n \"new\": \"say \\\"}\\\" \\\\\"}";
    let reply = format!("**@tool sys:edit {json}**\n@done");
    assert_eq!(parse_tool_call(&reply), call("sys", "edit", json));
}

#[test]
fn a_bullet_or_quote_marker_in_front_is_still_a_call() {
    assert_eq!(
        parse_tool_call("- @tool sys:list_dir {}"),
        call("sys", "list_dir", "{}")
    );
    assert_eq!(
        parse_tool_call("> `@tool sys:list_dir`"),
        call("sys", "list_dir", "")
    );
}

#[test]
fn json_that_never_closes_is_the_rest_of_the_line_as_before() {
    // A reply cut off by its token cap: the tool gets what there is, refuses
    // it, and the model is shown why — rather than the pane showing the call.
    assert_eq!(
        parse_tool_call("@tool a:b {\"x\": 1"),
        call("a", "b", "{\"x\": 1")
    );
}

#[test]
fn a_call_quoted_in_an_explanation_is_not_a_call() {
    let reply = "To read a file, agent smith writes:\n```\n\
                 @tool sys:read_file {\"path\": \"src/main.rs\"}\n```\n\
                 The broker runs it and sends the result back before the model answers.\n\
                 The path is relative to the project root.\n\
                 A file over 5,600 bytes comes back in pages, each saying where the next starts.";
    assert_eq!(parse_tool_call(reply), None);
    // One line, but a whole explanation's worth of it.
    let long = format!(
        "```\n@tool sys:read_file {{\"path\": \"x\"}}\n```\n{}",
        "That is how a file is read, and the result goes back to the model. ".repeat(3)
    );
    assert_eq!(parse_tool_call(&long), None);
}

#[test]
fn a_plain_answer_is_not_a_call() {
    assert_eq!(
        parse_tool_call("The build passes.\nNothing to change.\n@done"),
        None
    );
    // `@tool` mentioned in prose is not a line that starts with it.
    assert_eq!(
        parse_tool_call("Agents call tools with a `@tool` line."),
        None
    );
}

#[test]
fn depth_ignores_a_quote_before_the_value_opens() {
    let mut d = JsonDepth::default();
    assert!(!"say \"".chars().any(|c| d.push(c)));
    assert!(!d.open() && !d.closed());
    assert!("{\"a\": [1]}".chars().any(|c| d.push(c)));
    assert!(d.closed() && d.push('x'));
}

/// The text a split leaves is what the model wrote before the call: nothing
/// of the call, its JSON, its fence or what trailed it may survive.
#[test]
fn a_split_takes_a_fenced_multi_line_call_out_whole() {
    let reply = "I found the clip bug.\n\n```json\n@tool sys:read_file {\n  \
                 \"path\": \"a.rs\"\n}\n```\nReading its tests next.\n@done";
    let (before, c) = split_tool_call(reply).unwrap();
    assert_eq!(before, "I found the clip bug.");
    assert_eq!(
        c,
        call("sys", "read_file", "{\n  \"path\": \"a.rs\"\n}").unwrap()
    );
}

#[test]
fn a_split_of_a_reply_that_is_only_a_call_leaves_nothing() {
    let (before, _) = split_tool_call("```\n@tool sys:list_dir {}\n```").unwrap();
    assert_eq!(before, "");
    let (before, _) = split_tool_call("@tool sys:read_file\n{\"path\": \"x\"}").unwrap();
    assert_eq!(before, "");
}

/// A fence line above the call is the call's only when the call is inside
/// it. One that CLOSES an earlier block is the model's text, and taking it
/// would leave that block open over everything under it.
#[test]
fn a_split_keeps_a_fence_that_closed_an_earlier_block() {
    let reply = "The fix:\n```rust\nlet x = 1;\n```\n@tool sys:run {\"cmd\": \"cargo test\"}";
    let (before, _) = split_tool_call(reply).unwrap();
    assert_eq!(before, "The fix:\n```rust\nlet x = 1;\n```");
}

#[test]
fn a_split_is_none_wherever_a_parse_is() {
    assert_eq!(split_tool_call("The build passes.\n@done"), None);
    assert_eq!(
        split_tool_call("@tool fs:read {}\nactually, never mind"),
        None
    );
}

/// The exchange log's own spelling of a call, copied for the next one.
#[test]
fn a_call_written_the_way_the_log_writes_calls_is_a_call() {
    let reply = "It prints hello.\n\nNow, to count the lines of `notes.txt`, I will read it.\n\
                 CALLED sys:read_file {\"path\": \"notes.txt\"}";
    let (said, calls) = split_tool_calls(reply).expect("a call");
    assert_eq!(
        Some(calls[0].clone()),
        call("sys", "read_file", "{\"path\": \"notes.txt\"}")
    );
    assert!(
        !said.contains("CALLED"),
        "the call is cut from what is said: {said:?}"
    );
    // Only that exact spelling: prose that says it called something is not.
    assert_eq!(
        parse_tool_call("I called sys:read_file and it was empty."),
        None
    );
    assert_eq!(
        parse_tool_call("Called sys:read_file {\"path\": \"a\"}"),
        None
    );
}

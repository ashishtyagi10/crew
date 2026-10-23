use crate::tools::{split_tool_calls, ToolCall};

/// The capture, as qwen3.8-max wrote it (the `</tool_call>` never comes:
/// crew stops the stream on it).
#[test]
fn the_captured_call_runs() {
    let reply = "I have all the locations. Let me make the edits now.\n\n<tool_call>\n\
                 <parameter name=\"cmd\">sys:edit</parameter>\n\
                 <parameter name=\"args\">{\"path\": \"lib.rs\", \"old\": \"fn a\", \"new\": \"fn b\"}</parameter>";
    let (said, calls) = split_tool_calls(reply).expect("a call");
    assert_eq!(said, "I have all the locations. Let me make the edits now.");
    let want = ToolCall {
        server: "sys".into(),
        tool: "edit".into(),
        args: "{\"path\": \"lib.rs\", \"old\": \"fn a\", \"new\": \"fn b\"}".into(),
    };
    assert_eq!(calls, vec![want]);
}

/// One parameter per argument under the name, Anthropic's XML way.
#[test]
fn each_argument_a_parameter_of_its_own() {
    let reply = "<tool_call>\n<parameter name=\"name\">sys:read_file</parameter>\n\
                 <parameter name=\"path\">a.rs</parameter>\n\
                 <parameter name=\"lines\">40</parameter>\n</tool_call>";
    let (_, calls) = split_tool_calls(reply).expect("a call");
    assert_eq!(calls[0].tool, "read_file");
    assert_eq!(calls[0].args, "{\"path\": \"a.rs\", \"lines\": \"40\"}");
}

/// Parameters that name no tool, or arguments that are not JSON, are text.
#[test]
fn a_block_that_names_no_tool_stays_text() {
    let no_name = "<tool_call>\n<parameter name=\"path\">a.rs</parameter>\n</tool_call>";
    assert!(split_tool_calls(no_name).is_none());
    let bad = "<tool_call>\n<parameter name=\"cmd\">sys:edit</parameter>\n\
               <parameter name=\"args\">{\"path\": </parameter>";
    assert!(split_tool_calls(bad).is_none());
}

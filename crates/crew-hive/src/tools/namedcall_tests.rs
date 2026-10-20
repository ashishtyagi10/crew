use crate::tools::split_tool_calls;

/// The four blocks qwen3.8-max wrote, as they arrived: the stop at
/// `</tool_call>` leaves each block open.
const LIVE: [&str; 4] = [
    "Reading it.\n\n<tool_call>\n<tool_name>sys:read_file</tool_name>\n<arguments>{\"path\": \"crates/a.rs\", \"line\": 150}</arguments>",
    "Searching.\n\n<tool_call>\n<function_name>sys:grep</function_name>\n<arguments>{\"pattern\": \"relative_luminance\", \"path\": \"crates\"}</arguments>",
    "<tool_call>\nsys:edit {\"path\": \"a.rs\", \"old\": \"x\", \"new\": \"y\"}",
    "<tool_call>\n<function_name>sys:read_file</function_name>\n<arg_key>path</arg_key>\n<arg_value>crates/a.rs</arg_value>\n<arg_key>line</arg_key>\n<arg_value>4</arg_value>",
];

#[test]
fn every_shape_qwen3_8_max_wrote_runs() {
    let want: [(&str, &str); 4] = [
        (
            "sys:read_file",
            "{\"path\": \"crates/a.rs\", \"line\": 150}",
        ),
        (
            "sys:grep",
            "{\"pattern\": \"relative_luminance\", \"path\": \"crates\"}",
        ),
        (
            "sys:edit",
            "{\"path\": \"a.rs\", \"old\": \"x\", \"new\": \"y\"}",
        ),
        // Every pair value is text, as Qwen3-Coder's parameters are.
        (
            "sys:read_file",
            "{\"path\": \"crates/a.rs\", \"line\": \"4\"}",
        ),
    ];
    for (reply, (label, args)) in LIVE.iter().zip(want) {
        let (_, calls) = split_tool_calls(reply).unwrap_or_else(|| panic!("no call in {reply:?}"));
        assert_eq!(calls.len(), 1, "{reply:?}");
        assert_eq!(
            (calls[0].label().as_str(), calls[0].args.as_str()),
            (label, args)
        );
    }
}

#[test]
fn the_closing_tags_go_with_the_call() {
    let reply = "<tool_call>\n<tool_name>sys:git</tool_name>\n<arguments>{\"args\": \"status\"}</arguments>\n</tool_call>";
    let (said, calls) = split_tool_calls(reply).expect("a call");
    assert_eq!(said, "");
    assert_eq!(calls[0].label(), "sys:git");
}

#[test]
fn what_is_not_a_call_stays_text() {
    for reply in [
        "<tool_call>\nsys:edit\n</function>",
        "<tool_call>\n<tool_name>sys:grep</tool_name>\nno arguments here",
        "<tool_call>\nnotes: {\"a\": 1} is the shape",
        "<tool_call>\nhttps://example.com {\"a\": 1}",
        "<tool_call>\nsys:edit {\"path\": \"a.rs\"",
    ] {
        assert!(split_tool_calls(reply).is_none(), "{reply:?}");
    }
}

//! Several calls on a reply's last lines (`split_tool_calls`): read together
//! when they are consecutive, and only the last when prose sits between.
use super::*;

fn read(path: &str) -> ToolCall {
    ToolCall {
        server: "sys".into(),
        tool: "read_file".into(),
        args: format!("{{\"path\": \"{path}\"}}"),
    }
}

fn line(path: &str) -> String {
    format!("@tool sys:read_file {{\"path\": \"{path}\"}}")
}

#[test]
fn three_trailing_calls_are_three_calls_in_the_order_written() {
    let reply = format!(
        "The route is split over three files.\n{}\n{}\n{}",
        line("a.rs"),
        line("b.rs"),
        line("c.rs")
    );
    let (said, calls) = split_tool_calls(&reply).unwrap();
    assert_eq!(said, "The route is split over three files.");
    assert_eq!(calls, vec![read("a.rs"), read("b.rs"), read("c.rs")]);
}

/// Prose under a call means the model was showing it, not making it: only
/// the call the reply ends with runs, and the earlier one stays in the text.
#[test]
fn a_call_with_prose_under_it_is_quoted_and_only_the_last_runs() {
    let reply = format!(
        "{}\nthat is how a file is read.\n{}",
        line("a.rs"),
        line("b.rs")
    );
    let (said, calls) = split_tool_calls(&reply).unwrap();
    assert_eq!(calls, vec![read("b.rs")]);
    assert_eq!(
        said,
        format!("{}\nthat is how a file is read.", line("a.rs"))
    );
}

#[test]
fn calls_in_one_fence_with_blank_lines_between_come_out_whole() {
    let reply = format!(
        "Reading both.\n```\n{}\n\n{}\n```\n@done",
        line("a.rs"),
        line("b.rs")
    );
    let (said, calls) = split_tool_calls(&reply).unwrap();
    assert_eq!(calls, vec![read("a.rs"), read("b.rs")]);
    assert_eq!(said, "Reading both.", "the fence goes with the calls");
}

#[test]
fn an_earlier_call_may_spread_its_json_over_lines_if_it_closes() {
    let json = "{\n  \"path\": \"a.rs\"\n}";
    let reply = format!("@tool sys:read_file {json}\n{}", line("b.rs"));
    let (said, calls) = split_tool_calls(&reply).unwrap();
    assert_eq!(said, "");
    assert_eq!(calls[0].args, json);
    assert_eq!(calls[1], read("b.rs"));
}

/// JSON that never closes above another call is a call never finished: it
/// is not run, and it stays in the text rather than disappearing.
#[test]
fn an_earlier_call_whose_json_never_closes_is_not_a_call() {
    let reply = format!("@tool sys:read_file {{\"path\": \"a.rs\"\n{}", line("b.rs"));
    let (said, calls) = split_tool_calls(&reply).unwrap();
    assert_eq!(calls, vec![read("b.rs")]);
    assert!(said.contains("a.rs"), "{said}");
}

#[test]
fn one_call_splits_as_the_single_form_splits_it() {
    let reply = "I found it.\n```json\n@tool sys:read_file {\"path\": \"a.rs\"}\n```\nNext.";
    let (said, calls) = split_tool_calls(reply).unwrap();
    let (one_said, one) = split_tool_call(reply).unwrap();
    assert_eq!((said, calls), (one_said, vec![one]));
    assert_eq!(split_tool_calls("The build passes.\n@done"), None);
}

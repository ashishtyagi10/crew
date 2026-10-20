use super::super::split_tool_calls;
use super::cut;

/// The live answer's shape: a call, the result the model made up for it, a
/// second call and its made-up result, then a claim of success.
const NARRATED: &str = "Now the usages in glassink_tests.rs.\n\
CALLED sys:edit {\"path\": \"a.rs\", \"old\": \"x\", \"new\": \"y\"}\n\
RESULT:\n\
edited a.rs at line 4 (1 line)\n\
\n\
CALLED sys:run {\"cmd\": \"cargo test\"}\n\
RESULT:\n\
test result: ok\n\
\n\
All occurrences have been renamed and the tests pass.";

#[test]
fn the_first_narrated_call_runs_and_what_it_invented_is_dropped() {
    let (said, calls) = split_tool_calls(NARRATED).expect("the narrated call runs");
    assert_eq!(said, "Now the usages in glassink_tests.rs.");
    assert_eq!(
        calls.len(),
        1,
        "the second call stood on an invented result"
    );
    assert_eq!(calls[0].label(), "sys:edit");
    assert!(calls[0].args.contains("\"a.rs\""), "{}", calls[0].args);
}

#[test]
fn a_result_on_the_calls_own_line_is_invented_too() {
    let call = "Reading it.\n@tool sys:read_file {\"path\": \"a.rs\"}";
    let reply = format!("{call}\nRESULT: 1│ fn a() {{}}\nThe file defines a(), as expected.");
    assert_eq!(cut(&reply), call);
    let (_, calls) = split_tool_calls(&reply).expect("the call runs");
    assert_eq!(calls[0].label(), "sys:read_file");
}

#[test]
fn a_call_whose_json_spans_lines_is_cut_under_its_closing_brace() {
    let reply = "CALLED sys:edit {\n  \"path\": \"a.rs\",\n  \"old\": \"x\",\n  \"new\": \"y\"\n}\nRESULT:\nok";
    assert_eq!(
        cut(reply),
        "CALLED sys:edit {\n  \"path\": \"a.rs\",\n  \"old\": \"x\",\n  \"new\": \"y\"\n}"
    );
}

#[test]
fn an_answer_that_only_mentions_results_is_left_alone() {
    for reply in [
        "The RESULT: line in the log is the tool's output.",
        "CALLED is how the log spells a call.\nRESULT: is its output.",
        "Fixed it.\n\nRESULT:\nall green",
    ] {
        assert_eq!(cut(reply), reply);
    }
}

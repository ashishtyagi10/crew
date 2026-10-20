/// The judge reads the request, the calls and the answer, and is told a claim
/// no tool result shows does not count.
#[test]
fn the_judge_reads_what_the_tools_did_beside_what_the_answer_says() {
    let p = super::prompt(
        "rename it and make sure the tests pass",
        "1. sys:edit {\"path\": \"a.rs\"} \u{2192} ok: edited a.rs",
        "Renamed everywhere; all tests pass.",
    );
    let (ask, rest) = p.split_once("What the agent's tools did").unwrap();
    assert!(ask.contains("make sure the tests pass"), "{p}");
    let (tools, rest) = rest.split_once("The agent's answer:").unwrap();
    assert!(tools.contains("sys:edit"), "{p}");
    assert!(rest.contains("all tests pass"), "{p}");
    assert!(p.contains("not merely claimed"), "{p}");
    assert!(
        p.ends_with("`MET: <why>` or `NOT MET: <what is missing>`."),
        "{p}"
    );
}

#[test]
fn a_turn_with_no_calls_says_so_rather_than_nothing() {
    let p = super::prompt("make the tests pass", "  ", "Done.");
    assert!(p.contains("in order:\n(no tool calls)\n"), "{p}");
}

/// The second turn leads with the gap and still carries the whole request.
#[test]
fn the_revision_names_the_gap_then_the_request() {
    let r = super::revise("rename x to y; tests must pass", "the tests were never run");
    assert!(
        r.starts_with("A check of your last turn found: the tests were never run\n"),
        "{r}"
    );
    assert!(
        r.ends_with("THE REQUEST:\nrename x to y; tests must pass"),
        "{r}"
    );
}

#[test]
fn a_verdict_line_is_the_word_and_the_reason() {
    assert_eq!(
        super::said("verified", " the tests ran green "),
        "verified \u{2014} the tests ran green"
    );
    assert_eq!(super::said("verified", "MET"), "verified");
    assert_eq!(
        super::said("not yet", "glasslegend_tests.rs still uses the old name"),
        "not yet \u{2014} glasslegend_tests.rs still uses the old name"
    );
}

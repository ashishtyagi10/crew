use super::*;

/// A finished answer that still holds a call says the call never ran
/// (baseline probe 2026-10-16: a fenced `@tool sys:git` under `@done`).
#[test]
fn an_answer_still_holding_a_call_says_it_never_ran() {
    let a = "We will look at history:\n```json\n@tool sys:git {\"cmd\":\"log\"}\n```\nThen fix it.";
    let got = never_ran(a.to_string());
    assert!(got.starts_with(a), "the answer as written: {got}");
    assert!(
        got.ends_with(
            "not run \u{2014} the answer asks for `sys:git`, but the turn ended before any \
             call in it ran"
        ),
        "{got}"
    );
    assert_eq!(never_ran("Done.".into()), "Done.");
    assert_eq!(never_ran(got.clone()), got, "said once");
}

/// Under the budget's note — the checklist's below it or not — the line says
/// the budget stopped the call.
#[test]
fn under_the_budget_note_the_budget_stopped_it() {
    let a = "Edits:\n@tool sys:edit {\"path\": \"a.rs\"}\nthen the tests.\n\n\
             [tool budget spent \u{2014} 4 rounds of tool calls]\n\n\
             checklist: 1 of 2 done \u{2014} not done: run the tests";
    let got = never_ran(a.to_string());
    assert!(
        got.ends_with("but the tool budget was spent before it could run"),
        "{got}"
    );
}

#[test]
fn the_engines_notes_are_told_from_the_agents_lines() {
    assert!(is_note("checklist: 1 of 3 done \u{2014} not done: x"));
    assert!(is_note(
        "[tool budget spent \u{2014} 4 rounds of tool calls]"
    ));
    assert!(is_note("not run \u{2014} the answer asks for `a:b`"));
    assert!(!is_note("not running the tests yet"));
    assert!(!is_note("@done"));
}

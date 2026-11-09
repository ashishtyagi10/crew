use super::*;

/// A finished answer that still holds a call loses the call — fence, JSON
/// and all — and the line naming it is smith's, not the answer's (baseline
/// probe 2026-10-16: a fenced `@tool sys:git` under `@done`).
#[test]
fn a_call_left_in_an_answer_is_lifted_out_and_named() {
    let a = "We will look at history:\n```json\n@tool sys:git {\"cmd\":\"log\"}\n```\nThen fix it.";
    let (text, note) = not_run(a);
    assert_eq!(text, "We will look at history:\nThen fix it.");
    assert_eq!(
        note.as_deref(),
        Some("! not run: sys:git log \u{2014} the turn ended first")
    );
    assert_eq!(not_run("Done."), ("Done.".to_string(), None));
}

/// The engine with no smith row puts the line under the answer, once, and
/// a second engine lifting it again gets the same line back.
#[test]
fn never_ran_says_it_under_the_answer_once() {
    let a = "Reading it:\n@tool sys:read_file {\"path\": \"README.md\"}";
    let got = never_ran(a.to_string());
    assert_eq!(
        got,
        "Reading it:\n\n! not run: sys:read_file README.md \u{2014} the turn ended first"
    );
    assert_eq!(never_ran(got.clone()), got, "said once");
    let (text, note) = not_run(&got);
    assert_eq!(text, "Reading it:");
    assert!(note.unwrap().starts_with("! not run: sys:read_file"));
    assert_eq!(never_ran("Done.".into()), "Done.");
}

/// Under the budget's note — the checklist's below it or not — the line says
/// the budget stopped the calls, and the notes stay in the answer.
#[test]
fn under_the_budget_note_the_budget_stopped_it() {
    let a =
        "Edits:\n@tool sys:edit {\"path\": \"a.rs\"}\n@tool sys:run {\"cmd\": \"cargo test\"}\n\
             then the tests.\n\n\
             [tool budget spent \u{2014} 4 rounds of tool calls]\n\n\
             checklist: 1 of 2 done \u{2014} not done: run the tests";
    let (text, note) = not_run(a);
    assert!(!text.contains("@tool"), "{text}");
    assert!(text.contains("[tool budget spent"), "{text}");
    assert_eq!(
        note.as_deref(),
        Some("! not run: sys:edit a.rs \u{b7} sys:run cargo test \u{2014} the tool budget ran out first")
    );
}

/// A fenced block of the agent's own code stays: only a fence the lift left
/// empty goes, and two real blocks side by side keep both their fences.
#[test]
fn only_a_fence_the_lift_emptied_goes() {
    let a = "```\na\n```\n\n```\nb\n```\n```sh\n@tool sys:run {\"cmd\": \"ls\"}\n```\nend";
    let (text, _) = not_run(a);
    assert_eq!(text, "```\na\n```\n\n```\nb\n```\nend");
}

#[test]
fn the_engines_notes_are_told_from_the_agents_lines() {
    assert!(is_note("checklist: 1 of 3 done \u{2014} not done: x"));
    assert!(is_note(
        "[tool budget spent \u{2014} 4 rounds of tool calls]"
    ));
    assert!(is_note("! not run: a:b \u{2014} the turn ended first"));
    assert!(!is_note("not running the tests yet"));
    assert!(!is_note("@done"));
}

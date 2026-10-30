//! A turn that ends with plan steps undone says which; one that finished
//! its plan says nothing.
use crew_hive::tools::todowire::{Step, StepState};

use super::left_note;
use StepState::*;

fn plan(states: &[StepState]) -> Vec<Step> {
    let names = [
        "Read the code",
        "Find the cause",
        "Run the tests",
        "Fix it",
        "Re-run",
    ];
    states
        .iter()
        .zip(names)
        .map(|(&state, text)| Step {
            text: text.into(),
            state,
        })
        .collect()
}

#[test]
fn the_first_step_left_is_named_and_the_rest_counted() {
    let p = plan(&[Done, Done, Active, Pending, Pending]);
    assert_eq!(
        left_note(&p).as_deref(),
        Some("\u{25cb} plan 2/5 \u{b7} left: Run the tests, +2 more")
    );
    let one = plan(&[Done, Pending, Done]);
    assert_eq!(
        left_note(&one).as_deref(),
        Some("\u{25cb} plan 2/3 \u{b7} left: Find the cause")
    );
}

#[test]
fn a_finished_plan_says_nothing() {
    assert_eq!(left_note(&plan(&[Done, Done, Done])), None);
    assert_eq!(left_note(&[]), None);
    // Claude Code's last step, still marked active as it writes the answer.
    assert_eq!(left_note(&plan(&[Done, Done, Active])), None);
    // But an active step with more after it is unfinished work.
    assert!(left_note(&plan(&[Done, Active, Pending])).is_some());
}

/// At the task's end the pane says it once, then the plan goes.
#[test]
fn the_pane_says_what_was_left_when_the_task_ends() {
    let mut p = crate::chat::tests::pane();
    p.absorb_task(7, true);
    p.checklist = plan(&[Done, Active, Pending]);
    p.absorb_task(7, false);
    assert!(p.checklist.is_empty(), "the plan goes with its run");
    let said: Vec<&str> = p.messages.iter().map(|m| m.text.as_str()).collect();
    assert!(
        said.iter()
            .any(|t| t.contains("plan 1/3 \u{b7} left: Find the cause, +1 more")),
        "{said:?}"
    );
}

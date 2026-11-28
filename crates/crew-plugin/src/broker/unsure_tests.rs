use super::{row, split_unsure, ASK};
use crate::broker::route::{frame, repair_prompt};
use crate::broker::Envelope;

fn unsure(a: &str) -> (String, Option<String>) {
    split_unsure(a)
}

#[test]
fn the_last_line_is_lifted_out_of_the_answer() {
    let (kept, said) = unsure("It works.\n\nUnsure: calc.py was never run");
    assert_eq!(kept, "It works.");
    assert_eq!(said.as_deref(), Some("calc.py was never run"));
}

#[test]
fn a_line_that_is_not_last_stays_in_the_answer() {
    let a = "Unsure: of the path\nThen more prose.";
    assert_eq!(unsure(a), (a.to_string(), None));
}

#[test]
fn a_line_inside_a_fence_is_a_quoted_example_not_a_caveat() {
    let a = "Write it like:\n```\nUnsure: x";
    assert_eq!(unsure(a), (a.to_string(), None));
    let closed = "Write it like:\n```\nUnsure: x\n```";
    assert_eq!(unsure(closed), (closed.to_string(), None));
}

#[test]
fn a_line_that_says_nothing_is_dropped_unreported() {
    for body in ["", " nothing", "None.", "N/A"] {
        let (kept, said) = unsure(&format!("Done.\nUnsure:{body}"));
        assert_eq!((kept.as_str(), said), ("Done.", None), "{body:?}");
    }
}

#[test]
fn the_label_is_case_insensitive_and_trimmed() {
    let (kept, said) = unsure("Done.\n  UNSURE:   the port  \n");
    assert_eq!(
        (kept.as_str(), said.as_deref()),
        ("Done.", Some("the port"))
    );
}

#[test]
fn a_control_line_after_it_is_kept_and_does_not_hide_it() {
    let (kept, said) = unsure("Done.\nUnsure: the port\n@done");
    assert_eq!(kept, "Done.\n@done");
    assert_eq!(said.as_deref(), Some("the port"));
    assert_eq!(unsure("Plain."), ("Plain.".to_string(), None));
    assert_eq!(unsure(""), (String::new(), None));
}

#[test]
fn smiths_row_wears_its_own_glyph() {
    assert_eq!(row("the port"), "? unsure: the port");
}

/// The working framings ask for the line; the one-shot judges, the router
/// and the drafters never do.
#[test]
fn the_working_framings_ask_and_the_one_shots_do_not() {
    let env = Envelope::new("user", "claude", "t", "go");
    let alone = frame(&env, Some(""), &[], "task", "", "");
    let peers = frame(&env, Some(""), &["codex".into()], "task", "", "");
    for p in [alone, peers, repair_prompt(&["codex".into()], "prev")] {
        assert!(p.contains(ASK), "{p}");
    }
    let none = |p: String| assert!(!p.contains("Unsure:"), "{p}");
    none(crate::broker::gitmsg::commit_prompt("diff"));
    none(crate::broker::gitprdraft::pr_prompt("main", "log", "diff"));
    none(crate::broker::review::review_prompt("diff", ""));
    none(crate::broker::plan::plan_prompt("task"));
    none(crate::broker::replyverify::prompt(
        "task", "proof", "answer",
    ));
}

/// What the answer was unsure of is one of smith's rows, beside the rest.
#[test]
fn an_unsure_line_is_a_row_of_smiths() {
    let u = crate::broker::adapter::Usage {
        unsure: row("the port"),
        ..Default::default()
    };
    let said = crate::broker::fellback::Told::default().line("claude", &u);
    let Some(crate::PluginEvent::Message { text, .. }) = said else {
        panic!("a row")
    };
    assert_eq!(text, "? unsure: the port");
}

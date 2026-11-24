//! The advisor before an answer stands (`advisor::done`), on a whole turn of
//! either engine: a scripted model that edits and answers is advised once and
//! answers again with the advice in front of it; one whose check passed after
//! its edit, or that edited nothing, or whose reads are spent, is not.
use crate::advisor::HEAD;
use kit::{turn, turn_with, Step, ADVICE};

#[path = "donekit_tests.rs"]
mod kit;

const EDIT_A: Step = ("edit", r#"{"path": "a.rs", "old": "x", "new": "y"}"#);
const EDIT_B: Step = ("edit", r#"{"path": "b.rs", "old": "x", "new": "y"}"#);
const TEST: Step = ("run", r#"{"cmd": "cargo test -p a"}"#);

#[tokio::test]
async fn an_answer_on_an_unchecked_edit_is_advised_once_and_answered_again() {
    for native in [false, true] {
        let (shown, briefs, said) = turn(native, &[EDIT_A]).await;
        assert_eq!(briefs.len(), 1, "native {native}");
        let b = &briefs[0];
        assert!(b.contains("about to end its turn"), "{b}");
        assert!(b.contains("SINCE THE LAST PASSING CHECK:\na.rs"), "{b}");
        assert!(b.contains("ITS LAST CHECK: none ran this turn"), "{b}");
        assert!(b.contains("ITS ANSWER:\ndone"), "{b}");
        assert_eq!(said, [format!("advised \u{b7} {ADVICE}")]);
        // Edit, the held answer, and the one more round the advice buys.
        assert_eq!(shown.len(), 3, "native {native}");
        assert!(!shown[1].contains(HEAD), "native {native}: {}", shown[1]);
        let last = &shown[2];
        assert!(last.contains(HEAD) && last.contains(ADVICE), "{last}");
        assert!(last.contains("give your final answer again"), "{last}");
    }
}

#[tokio::test]
async fn a_check_that_passed_after_the_edit_or_no_edit_asks_nothing() {
    for native in [false, true] {
        for steps in [&[EDIT_A, TEST][..], &[("run", r#"{"cmd": "ls"}"#)], &[]] {
            let (shown, briefs, said) = turn(native, steps).await;
            assert!(briefs.is_empty() && said.is_empty(), "{native} {steps:?}");
            assert_eq!(shown.len(), steps.len() + 1, "{native} {steps:?}");
        }
    }
}

#[tokio::test]
async fn an_edit_after_a_passing_check_is_read_with_that_check() {
    let (_, briefs, _) = turn(true, &[EDIT_A, TEST, EDIT_B]).await;
    assert_eq!(briefs.len(), 1);
    let b = &briefs[0];
    assert!(b.contains("SINCE THE LAST PASSING CHECK:\nb.rs\n"), "{b}");
    assert!(
        b.contains("`cargo test -p a` passed before the last change"),
        "{b}"
    );
}

#[tokio::test]
async fn a_failed_check_after_the_edit_is_read_once_not_twice() {
    for native in [false, true] {
        let fail = ("run", r#"{"cmd": "cargo build fail"}"#);
        let (shown, briefs, _) = turn(native, &[EDIT_A, fail]).await;
        assert_eq!(briefs.len(), 1, "native {native}");
        assert!(
            briefs[0].contains("THE COMMAND THAT FAILED"),
            "{}",
            briefs[0]
        );
        assert_eq!(shown.len(), 3, "native {native}: the answer stood");
    }
}

#[tokio::test]
async fn a_turn_whose_reads_are_spent_answers_unread() {
    let fails = [
        ("run", r#"{"cmd": "cargo build fail 1"}"#),
        ("run", r#"{"cmd": "cargo build fail 2"}"#),
    ];
    let (shown, briefs, _) = turn(true, &[fails[0], fails[1], EDIT_A]).await;
    assert_eq!(briefs.len(), crate::advisor::PER_TURN as usize);
    assert!(briefs.iter().all(|b| b.contains("THE COMMAND THAT FAILED")));
    assert_eq!(shown.len(), 4, "the answer stood");
}

#[tokio::test]
async fn with_no_advisor_configured_the_answer_stands() {
    for native in [false, true] {
        let (shown, briefs, said) = turn_with(native, &[EDIT_A], false).await;
        assert!(briefs.is_empty() && said.is_empty(), "native {native}");
        assert_eq!(shown.len(), 2, "native {native}");
    }
}

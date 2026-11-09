//! The stop check on a whole turn: held once, it costs one more model call
//! with the reason in it; held for ever, it ends at the cap or as soon as
//! the answer stops changing, with a failed row saying so; broken, it ends
//! the turn with a failed row; and silent, it changes nothing.
use super::super::Ended;
use super::kit::{turn, turn_on};
use super::MAX_HOLDS;
use crate::provider::Turn;

#[tokio::test]
async fn held_once_the_turn_takes_one_more_call_with_the_reason_in_it() {
    let ran = turn(Some(1), true).await;
    assert_eq!(ran.requests.len(), 2);
    assert_eq!(ran.ended, Ended::Answered("done 2".into()));
    let last = &ran.requests[1].turns;
    assert!(matches!(&last[0], Turn::Assistant { text, .. } if text == "done 1"));
    assert!(matches!(&last[1], Turn::User(n) if n.contains("the tests fail")));
    assert_eq!(
        ran.flags,
        [false, true],
        "the second check knows it held once"
    );
    assert_eq!(
        ran.rows,
        [(true, "held the turn (1 of 3): the tests fail".into())]
    );
}

#[tokio::test]
async fn held_for_ever_the_turn_ends_at_the_cap_with_a_failed_row() {
    let ran = turn(Some(u32::MAX), true).await;
    assert_eq!(ran.requests.len(), 1 + MAX_HOLDS as usize);
    assert_eq!(
        ran.ended,
        Ended::Answered(format!("done {}", 1 + MAX_HOLDS))
    );
    let (ok, last) = ran.rows.last().unwrap();
    assert!(!ok && last.contains("held 3 times already") && last.contains("tests fail"));
}

#[tokio::test]
async fn an_answer_that_does_not_change_is_not_held_again() {
    let ran = turn(Some(u32::MAX), false).await;
    assert_eq!(ran.requests.len(), 2);
    let (ok, last) = ran.rows.last().unwrap();
    assert!(!ok && last.contains("did not change"), "{last}");
}

#[tokio::test]
async fn a_broken_check_ends_the_turn_with_a_failed_row() {
    let ran = turn(None, true).await;
    assert_eq!(ran.requests.len(), 1);
    assert_eq!(
        ran.rows,
        [(false, "ran past its 5s deadline and was stopped".into())]
    );
}

#[tokio::test]
async fn a_check_that_lets_it_end_adds_no_call_and_no_row() {
    let ran = turn(Some(0), true).await;
    assert_eq!(ran.requests.len(), 1);
    assert!(ran.rows.is_empty());
}

/// The `@tool` loop holds the same way: the answer and the reason go back in
/// the next prompt, since that loop has no turns to append them to.
#[tokio::test]
async fn the_text_loop_holds_with_the_answer_and_reason_in_the_next_prompt() {
    let ran = turn_on(Some(1), true, false).await;
    assert_eq!(ran.requests.len(), 2);
    let next = &ran.requests[1].prompt;
    assert!(next.contains("YOUR ANSWER WAS:\ndone 1") && next.contains("the tests fail"));
    assert_eq!(ran.ended, Ended::Answered("done 2".into()));
}

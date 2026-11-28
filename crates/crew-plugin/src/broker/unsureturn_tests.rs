//! The `Unsure:` line through a real relay thread: lifted into the Done hop's `Usage` (a
//! `done()` that dropped it would pass every parser test), and never forwarded on a hand-off.
use crate::broker::turnkit::{turn, Script};
use crate::broker::HopKind;

#[test]
fn the_done_hop_carries_the_lifted_line_and_the_answer_does_not() {
    let t = turn(
        Script::new(&["it is 4\nUnsure: read from memory, not the file\n@done"]),
        |b| b,
        "task",
    );
    assert_eq!(t.answer(), Some("it is 4"));
    let done = t.hops.iter().find(|h| h.kind == HopKind::Done).unwrap();
    assert_eq!(
        done.usage.unsure,
        "? unsure: read from memory, not the file"
    );
}

#[test]
fn markdown_bold_is_the_same_line() {
    let t = turn(
        Script::new(&["ok\n**Unsure:** the env var name\n@done"]),
        |b| b,
        "t",
    );
    assert_eq!(t.answer(), Some("ok"));
    let done = t.hops.iter().find(|h| h.kind == HopKind::Done).unwrap();
    assert_eq!(done.usage.unsure, "? unsure: the env var name");
}

#[test]
fn a_hand_off_body_is_forwarded_without_the_line() {
    let body = super::strip("draft\nUnsure: maybe".to_string());
    assert_eq!(body, "draft");
    assert_eq!(super::strip("draft".to_string()), "draft");
    assert_eq!(super::strip("x\nUnsure: none".to_string()), "x");
}

#[test]
fn a_tilde_fence_quotes_the_line_too() {
    let a = "example:\n~~~\nUnsure: quoted";
    assert_eq!(super::split_unsure(a), (a.to_string(), None));
}

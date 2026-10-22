use std::sync::mpsc;

use winit::keyboard::{Key, NamedKey};

use crate::inbox::composekeys::tests::{reading, typed};
use crate::inbox::keys::InboxAction;
use crate::inbox::worker::{Got, Worker};
use crate::inbox::Stage;

#[test]
fn a_reply_waits_out_an_ended_sign_in_and_the_setup_page_keeps_its_keys() {
    let _g = crate::app::motion_test_guard();
    let (mut p, _) = reading();
    typed(&mut p, "r");
    typed(&mut p, "Half a reply");
    p.send_reply();
    assert!(p.compose.as_ref().unwrap().sending);
    p.take(Got::Ended("invalid_grant".into()));
    assert!(matches!(p.stage, Stage::Setup { .. }));
    let c = p.compose.as_ref().expect("the words are kept");
    assert!(!c.sending, "not stuck sending");
    // The page says the reply is kept, and Esc lets it go only when asked
    // twice (one Esc closed the pane, and the reply went with it).
    let page: String = p.cells(80, 30).iter().map(|c| c.c).collect();
    assert!(page.contains("is kept: it opens again"), "{page}");
    let esc = p.press(&Key::Named(NamedKey::Escape), 80, 30);
    assert!(matches!(esc, Some(InboxAction::Redraw)), "kept, said");
    assert!(p.note.contains("esc again discards it"), "{}", p.note);
    assert!(p.compose.is_some());
    typed(&mut p, "x");
    assert_eq!(
        p.compose.as_ref().unwrap().body.text,
        "Half a reply",
        "no key reaches the box"
    );
    p.send_reply();
    assert!(
        !p.compose.as_ref().unwrap().sending,
        "nothing to send it with"
    );
    let esc = p.press(&Key::Named(NamedKey::Escape), 80, 30);
    assert!(matches!(esc, Some(InboxAction::Close)), "asked twice");
    // Signed in again: the box and its words, as they were.
    let ((tx, _), (_, rx)) = (mpsc::channel(), mpsc::channel());
    p.stage = Stage::Ready(Worker { tx, rx });
    typed(&mut p, "!");
    assert_eq!(p.compose.as_ref().unwrap().body.text, "Half a reply!");
}

#[test]
fn a_reply_on_its_way_keeps_its_box_until_gmail_says_it_went() {
    let _g = crate::app::motion_test_guard();
    let (mut p, _) = reading();
    typed(&mut p, "r");
    typed(&mut p, "Yes.");
    p.send_reply();
    for _ in 0..2 {
        p.press(&Key::Named(NamedKey::Escape), 80, 30);
    }
    assert!(p.compose.is_some(), "Esc does not let a sending box go");
    assert!(p.note.contains("on its way"), "{}", p.note);
    p.take(Got::Sent("ann@x.org".into()));
    assert!(p.compose.is_none(), "Gmail said it went");
}

/// Only the send's own answer unlocks the box. A check asked just before
/// Cmd+Enter is answered first, and when it failed it unlocked the reply
/// still on its way: a second Cmd+Enter sent it twice.
#[test]
fn another_call_failing_does_not_unlock_a_reply_on_its_way() {
    let _g = crate::app::motion_test_guard();
    let (mut p, _) = reading();
    typed(&mut p, "r");
    typed(&mut p, "Yes.");
    p.send_reply();
    p.take(Got::Failed("Gmail 503".into()));
    assert!(p.compose.as_ref().unwrap().sending, "still on its way");
    p.take(Got::NotSent("Gmail 500".into()));
    let c = p.compose.as_ref().unwrap();
    assert!(!c.sending && c.body.text == "Yes.", "kept, to try again");
    assert!(p.note.contains("not sent"), "{}", p.note);
}

/// A forward is not a load: once it went, the minute's check and `j`'s
/// next page go on (it left the pane busy for good).
#[test]
fn a_forward_leaves_the_list_live() {
    use crate::inbox::worker::Ask;
    let _g = crate::app::motion_test_guard();
    let (mut p, asks) = reading();
    p.busy = false;
    typed(&mut p, "f");
    typed(&mut p, "kim@x.org");
    p.send_reply();
    assert!(asks.try_iter().any(|a| matches!(a, Ask::Mail(_))), "sent");
    p.take(Got::Forwarded("kim@x.org".into()));
    assert!(!p.busy, "the list stays live");
}

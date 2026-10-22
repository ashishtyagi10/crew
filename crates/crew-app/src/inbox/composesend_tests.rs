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
    let esc = p.press(&Key::Named(NamedKey::Escape), 80, 30);
    assert!(
        matches!(esc, Some(InboxAction::Close)),
        "the setup page's Esc"
    );
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

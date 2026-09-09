//! Done-means 3 of the JARVIS goal, as one test: **the same task, through all three channels,
//! reaches the same agent with the same words.**
//!
//! That is the whole claim a `Channel` trait makes. If a pane, a phone and a microphone are the
//! same kind of thing, then a task typed here, sent from a phone, or spoken aloud must arrive
//! identically — same session shape, same requester class, same text — and differ only in where
//! the answer goes. A voice channel that quietly prefixed "user said:" would pass every test in
//! `voice/` and fail this one.
use super::rig::rig;
use crate::channel::{loopback::Loopback, Inbound};
use crate::daemon::answer;
use crate::ipc_types::{Reply, Request, PROTOCOL_V};
use crate::voice::{Ears, Mouth, Speech};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

const TASK: &str = "what is on my calendar tomorrow";

/// A microphone that always hears the same sentence.
struct SaidIt(Vec<i16>);
impl Ears for SaidIt {
    fn record(&self, _stop: &AtomicBool) -> Result<Vec<i16>, String> {
        Ok(self.0.clone())
    }
}
struct Deaf;
impl Mouth for Deaf {
    fn play(&self, _pcm: &[i16], _rate: u32, _stop: &AtomicBool) -> Result<(), String> {
        Ok(())
    }
}
struct Heard(Arc<Mutex<Vec<String>>>);
impl Speech for Heard {
    fn transcribe(&self, _wav: &[u8]) -> Result<String, String> {
        Ok(TASK.to_string())
    }
    fn speak(&self, text: &str) -> Result<Vec<i16>, String> {
        self.0.lock().unwrap().push(text.to_string());
        Ok(vec![0; 8])
    }
}

fn loud() -> Vec<i16> {
    (0..16_000)
        .map(|i| ((i as f32 / 8.0).sin() * 6_000.0) as i16)
        .collect()
}

/// What the session was told, whatever channel it came from: the requester the broker child was
/// opened for, and the text written into it.
fn through_a_channel(tag: &str, from: &str) -> (Vec<String>, Vec<String>) {
    let mut r = rig(tag);
    r.wire.lock().unwrap().inbox.push(Inbound {
        from: from.into(),
        text: TASK.into(),
    });
    r.d.service_channels();
    let opened = r.opened.lock().unwrap().clone();
    (opened, r.tasks())
}

#[test]
fn a_typed_task_and_a_phone_task_reach_the_same_kind_of_session() {
    // The loopback channel stands in for a pane and for a phone: both are a `Channel` handing
    // crew a line of text from an address, which is exactly what Telegram does.
    let (pane_opened, pane_said) = through_a_channel("parity-pane", "test:me");
    let (phone_opened, phone_said) = through_a_channel("parity-phone", "test:+15551234");
    assert_eq!(pane_opened, ["channel:test:me"]);
    assert_eq!(phone_opened, ["channel:test:+15551234"]);
    assert_eq!(pane_said, [TASK]);
    assert_eq!(
        pane_said, phone_said,
        "the same words reach the agent whichever channel carried them"
    );
}

#[test]
fn a_spoken_task_reaches_the_same_session_with_the_same_words() {
    let said = Arc::new(Mutex::new(Vec::new()));
    let mut r = rig("parity-voice");
    r.d.add_channel(Box::new(crate::voice::Voice::new(
        Arc::new(SaidIt(loud())),
        Arc::new(Deaf),
        Some(Arc::new(Heard(Arc::clone(&said)))),
    )));
    // Press the button, wait for the transcript to land, then let the daemon route it.
    match answer(
        &Request::Press {
            v: PROTOCOL_V,
            kind: "voice".into(),
        },
        &mut r.d,
    ) {
        Some(Reply::Pressed { .. }) => {}
        other => panic!("expected the button to answer, got {other:?}"),
    }
    for _ in 0..200 {
        r.d.service_channels();
        if !r.opened.lock().unwrap().is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(
        r.opened.lock().unwrap().as_slice(),
        ["channel:voice:local"],
        "a spoken task opens the same kind of session a typed one does"
    );
    assert_eq!(
        r.tasks().as_slice(),
        [TASK],
        "and it carries the words that were said, unchanged"
    );
}

#[test]
fn the_answer_goes_back_the_way_the_question_came() {
    // The one thing that SHOULD differ between the three: a spoken question is answered aloud,
    // and a typed one is not.
    let said = Arc::new(Mutex::new(Vec::new()));
    let mut r = rig("parity-answer");
    let (c, wire) = Loopback::pair("test");
    r.d.add_channel(Box::new(c));
    r.d.add_channel(Box::new(crate::voice::Voice::new(
        Arc::new(SaidIt(loud())),
        Arc::new(Deaf),
        Some(Arc::new(Heard(Arc::clone(&said)))),
    )));
    match answer(
        &Request::Say {
            v: PROTOCOL_V,
            to: "voice:local".into(),
            text: "the forecast is fine".into(),
        },
        &mut r.d,
    ) {
        Some(Reply::Sent { .. }) => {}
        other => panic!("expected it to be sent, got {other:?}"),
    }
    for _ in 0..200 {
        if !said.lock().unwrap().is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(said.lock().unwrap().as_slice(), ["the forecast is fine"]);
    assert!(
        wire.lock().unwrap().outbox.is_empty(),
        "and nothing was sent to the channel that did not ask"
    );
}

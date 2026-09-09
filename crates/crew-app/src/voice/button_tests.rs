//! What a press does, with no microphone, no key and no network: listen, stop, refuse silence,
//! report a failure, and interrupt.
use super::super::tests::{keyless, loud, rig, until};
use super::super::{State, ADDRESS};
use crate::channel::Channel;
use std::sync::atomic::Ordering;

#[test]
fn a_press_listens_and_what_was_said_arrives_as_a_message() {
    // The whole point: a spoken sentence becomes an ordinary inbound message, from an address
    // the daemon can answer.
    let mut r = rig(loud(), "what is on my calendar", false);
    r.voice.press().unwrap();
    assert!(until(
        || !r.voice.poll_ready().is_empty() || r.voice.state() == State::Idle
    ));
    let msgs = r.voice.poll();
    assert_eq!(msgs.len(), 1, "one utterance, one message");
    assert_eq!(msgs[0].text, "what is on my calendar");
    assert_eq!(msgs[0].from, ADDRESS, "and it can be answered");
}

#[test]
fn a_second_press_stops_the_recording() {
    let r = rig(loud(), "done talking", false);
    r.voice.press().unwrap();
    assert!(until(|| r.voice.state() == State::Listening));
    let what = r.voice.press().unwrap();
    assert!(what.contains("stopping"), "{what}");
    assert!(
        until(|| r.stopped.load(Ordering::SeqCst)),
        "the microphone was told to stop"
    );
    assert!(until(|| !r.voice.poll_ready().is_empty()));
}

#[test]
fn silence_costs_nothing_and_says_so() {
    // Whisper answers silence with a plausible sentence, which reads as crew hearing voices.
    let mut r = rig(vec![0; 16_000], "a hallucinated sentence", false);
    r.voice.press().unwrap();
    assert!(until(|| r.voice.state() == State::Idle));
    assert!(
        r.voice.poll().is_empty(),
        "nothing was said, so nothing was sent"
    );
    let notices = r.voice.notices();
    assert!(
        notices.iter().any(|n| n.contains("heard nothing")),
        "{notices:?}"
    );
}

#[test]
fn a_failed_transcription_is_a_notice_rather_than_a_task() {
    let mut r = rig(loud(), "", true);
    r.voice.press().unwrap();
    assert!(until(|| r.voice.state() == State::Idle));
    assert!(r.voice.poll().is_empty());
    let notices = r.voice.notices();
    assert!(
        notices.iter().any(|n| n.contains("service is down")),
        "{notices:?}"
    );
}

#[test]
fn pressing_while_crew_is_talking_stops_it_mid_sentence() {
    // Barge-in. A spoken answer you cannot interrupt is worse than a printed one.
    let mut r = rig(loud(), "never mind", false);
    r.voice.send(ADDRESS, "a very long answer").unwrap();
    assert!(until(|| r.voice.state() == State::Speaking));
    let what = r.voice.press().unwrap();
    assert!(what.contains("stopped talking"), "{what}");
    assert!(
        until(|| r.interrupted.load(Ordering::SeqCst)),
        "playback was cut off"
    );
    assert!(r.played.lock().unwrap().is_empty(), "and it never finished");
}

#[test]
fn a_press_with_no_key_tells_the_presser_why() {
    // The reason has to reach the terminal that pressed, not only the daemon's log: a button
    // that fails silently looks like a button that worked.
    let mut voice = keyless();
    let why = voice.press().unwrap_err();
    assert!(why.contains("OPENAI_API_KEY"), "{why}");
    let notices = voice.notices();
    assert!(
        notices.iter().any(|n| n.contains("OPENAI_API_KEY")),
        "{notices:?}"
    );
}

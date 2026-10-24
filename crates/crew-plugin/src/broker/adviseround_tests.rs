//! The advisor meeting a CLI agent's relay turn on the harness (`hopturn`):
//! a failed build is read before the follow-up is dialed, and the advice
//! rides that follow-up and the rest of the turn's; a stop pressed while the
//! build ran or while it was read still dials nothing more.
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use super::tests::{advising, install, Runs};
use super::text::HEAD;
use crate::broker::toolcall::ToolRunner;
use crate::broker::turnkit::{turn as relay, Script};
use crate::broker::Broker;

/// A turn on a broker `setup` gives its tools that builds (and fails), then
/// tests (and passes), then answers: its answer, and the follow-up prompts
/// it was dialed with.
fn turn(setup: impl FnOnce(Broker) -> Broker) -> (String, Vec<String>) {
    let replies = [
        "@tool sys:run {\"cmd\": \"cargo build\"}",
        "@tool sys:run {\"cmd\": \"cargo test\"}",
        "it builds\n@done",
    ];
    let t = relay(
        Script::new(&replies).named("engineer"),
        setup,
        "make it build",
    );
    let answer = t.answer().expect("an answer").to_string();
    (answer, t.prompts[1..].to_vec())
}

fn build_then_test() -> Vec<String> {
    let runs: Arc<dyn ToolRunner> = Arc::new(Runs);
    let (reply, prompts) = turn(|b| b.with_tools(runs));
    assert!(reply.contains("it builds"), "{reply}");
    prompts
}

#[test]
fn a_failed_build_is_advised_before_the_next_dial_and_for_the_rest_of_the_turn() {
    let (briefs, _on) = advising(Ok("E0308 at src/a.rs:3 \u{2014} fix the call site"));
    let prompts = build_then_test();
    assert_eq!(prompts.len(), 2);
    assert_eq!(
        briefs.lock().unwrap().len(),
        1,
        "the passing test asked nothing"
    );
    assert!(briefs.lock().unwrap()[0].contains("THE REQUEST:\nmake it build"));
    for p in &prompts {
        let (exchanges, added) = p.split_once(HEAD).expect("the advice rides the follow-up");
        assert!(exchanges.contains("TOOL EXCHANGES SO FAR"), "{p}");
        assert!(
            added.starts_with("\n- E0308 at src/a.rs:3 \u{2014} fix the call site\n\n"),
            "{p}"
        );
        assert!(
            added.contains("round(s) of tool calls"),
            "the budget line last: {p}"
        );
    }
}

#[test]
fn an_advisor_that_fails_leaves_the_follow_up_as_it_was() {
    let (briefs, _on) = advising(Err("timed out after 30s"));
    let prompts = build_then_test();
    assert_eq!(briefs.lock().unwrap().len(), 1);
    assert!(prompts.iter().all(|p| !p.contains(HEAD)), "{prompts:?}");
}

#[test]
fn a_turn_with_no_advisor_carries_no_section() {
    let prompts = build_then_test();
    assert!(prompts.iter().all(|p| !p.contains(HEAD)), "{prompts:?}");
}

/// [`Runs`], pressing stop as the build runs, as a person would.
struct Pressing(Arc<AtomicBool>);

impl ToolRunner for Pressing {
    fn hint(&self) -> String {
        Runs.hint()
    }
    fn call(&self, s: &str, t: &str, args: &str) -> Result<String, String> {
        self.0.store(true, Ordering::Relaxed);
        Runs.call(s, t, args)
    }
    fn failed(&self, s: &str, t: &str, out: &str) -> bool {
        Runs.failed(s, t, out)
    }
}

#[test]
fn a_stop_pressed_while_the_build_ran_is_not_read() {
    let (briefs, _on) = advising(Ok("unused"));
    let flag = Arc::new(AtomicBool::new(false));
    let runs = Arc::new(Pressing(Arc::clone(&flag)));
    let (reply, prompts) = turn(|b| b.with_tools(runs).with_cancel_flag(flag));
    assert!(briefs.lock().unwrap().is_empty(), "read after the stop");
    assert!(prompts.is_empty(), "dialed after the stop: {prompts:?}");
    assert_eq!(reply, "stopped \u{2014} 1 tool call made");
}

/// The read is a wait of its own, up to the one-shot bound: a stop pressed
/// during it is looked at before the follow-up goes out, not after.
#[test]
fn a_stop_pressed_while_the_advisor_read_dials_no_follow_up() {
    let flag = Arc::new(AtomicBool::new(false));
    let press = Arc::clone(&flag);
    let _on = install(Box::new(move |_: &str| {
        press.store(true, Ordering::Relaxed);
        Ok(("fix the call site".into(), crew_hive::Spent::default()))
    }));
    let runs = Arc::new(Runs);
    let (reply, prompts) = turn(|b| b.with_tools(runs).with_cancel_flag(flag));
    assert!(prompts.is_empty(), "dialed after the stop: {prompts:?}");
    assert_eq!(reply, "stopped \u{2014} 1 tool call made");
}

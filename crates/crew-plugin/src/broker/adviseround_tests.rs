//! The advisor meeting the relay's tool loop (`toolcall::run_tools`): a
//! failed build is read before the follow-up is dialed, and the advice rides
//! that follow-up and the rest of the turn's; a stop pressed while the build
//! ran or while it was read still dials nothing more.
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::tests::{advising, install, Runs};
use super::text::HEAD;
use crate::broker::adapter::{Adapter, HopStream, Usage};
use crate::broker::toolcall::ToolRunner;
use crate::broker::{Broker, Envelope, RunStats};
use crate::Registry;

/// An agent that answers from a script and keeps every prompt it was sent.
struct Agent(Mutex<Vec<&'static str>>, Mutex<Vec<String>>);

impl Adapter for Agent {
    fn name(&self) -> &str {
        "engineer"
    }
    fn probe(&self) -> bool {
        true
    }
    fn call(&self, body: &str, _t: Duration) -> Result<String, String> {
        self.1.lock().unwrap().push(body.to_string());
        Ok(self.0.lock().unwrap().remove(0).to_string())
    }
}

fn broker() -> Broker {
    Broker::new(Registry::new(vec![]), 6, Duration::from_secs(5))
}

/// A turn on `b` that builds (and fails), then tests (and passes), then
/// answers: its answer, and the follow-up prompts it was dialed with.
fn turn(b: Broker) -> (String, Vec<String>) {
    let replies = vec![
        "@tool sys:run {\"cmd\": \"cargo test\"}",
        "it builds\n@done",
    ];
    let agent = Agent(Mutex::new(replies), Mutex::default());
    let first = "@tool sys:run {\"cmd\": \"cargo build\"}".to_string();
    let env = Envelope::new("user", "engineer", "t1", "make it build");
    let reply = b.run_tools(
        &agent,
        "make it build",
        first,
        &mut RunStats::default(),
        &mut Usage::default(),
        &env,
        &HopStream::noop(),
        &mut |_| {},
    );
    let prompts = agent.1.lock().unwrap().clone();
    (reply, prompts)
}

fn build_then_test() -> Vec<String> {
    let (reply, prompts) = turn(broker().with_tools(Arc::new(Runs)));
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
        assert!(exchanges.contains("TOOL EXCHANGES THIS TURN"), "{p}");
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
    let (reply, prompts) = turn(broker().with_tools(runs).with_cancel_flag(flag));
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
    let (reply, prompts) = turn(broker().with_tools(runs).with_cancel_flag(flag));
    assert!(prompts.is_empty(), "dialed after the stop: {prompts:?}");
    assert_eq!(reply, "stopped \u{2014} 1 tool call made");
}

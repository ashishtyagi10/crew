//! A steer meeting the relay's tool loop (`toolcall::run_tools`): taken
//! between rounds, announced before the dial that carries it, and kept in
//! every follow-up for the rest of the turn. The inbox itself is tested in
//! `steer_tests`, whose turn-taking these share.
use std::time::Duration;

use super::tests::{offer, taking, turn, Log};
use super::*;
use crate::broker::adapter::{Adapter, HopStream, Usage};
use crate::broker::{Broker, Envelope, RunStats};
use crate::Registry;

/// An agent that answers from a script and writes each dial to the log.
struct Agent(Mutex<Vec<&'static str>>, Mutex<Vec<String>>, Log);

impl Adapter for Agent {
    fn name(&self) -> &str {
        "planner"
    }
    fn probe(&self) -> bool {
        true
    }
    fn call(&self, body: &str, _t: Duration) -> Result<String, String> {
        self.1.lock().unwrap().push(body.to_string());
        self.2.lock().unwrap().push("dial".into());
        Ok(self.0.lock().unwrap().remove(0).to_string())
    }
}

/// A read that runs `during` while it works — the user typing mid-round.
struct Tools(fn());

impl crate::broker::toolcall::ToolRunner for Tools {
    fn hint(&self) -> String {
        "TOOLS: fs:read".into()
    }
    fn call(&self, _s: &str, _t: &str, _a: &str) -> Result<String, String> {
        (self.0)();
        Ok("FILE".into())
    }
}

/// Run one tool turn on this thread as a task: the prompts the agent was
/// dialed with, and the log of announcements and dials, in order.
fn run_turn(during: fn()) -> (Vec<String>, Vec<String>) {
    let log = Log::default();
    let _task = taking(&log);
    let b = Broker::new(Registry::new(vec![]), 6, Duration::from_secs(5))
        .with_tools(Arc::new(Tools(during)));
    let replies = vec!["@tool fs:read {\"path\": \"b\"}", "checked both\n@done"];
    let agent = Agent(Mutex::new(replies), Mutex::default(), Arc::clone(&log));
    let first = "@tool fs:read {\"path\": \"a\"}".to_string();
    let env = Envelope::new("user", "planner", "t1", "task");
    let reply = b.run_tools(
        &agent,
        "task",
        first,
        &mut RunStats::default(),
        &mut Usage::default(),
        &env,
        &HopStream::noop(),
        &mut |_| {},
    );
    assert!(reply.contains("checked both"), "{reply}");
    let prompts = agent.1.lock().unwrap().clone();
    let order = log.lock().unwrap().clone();
    (prompts, order)
}

#[test]
fn a_steer_typed_during_a_round_joins_the_next_prompt_and_stays_for_the_turn() {
    let _t = turn();
    let (prompts, order) = run_turn(|| offer("also check the tests"));
    assert_eq!(prompts.len(), 2);
    for p in &prompts {
        let (exchanges, added) = p
            .split_once(HEAD)
            .expect("the section rides every follow-up");
        assert!(
            exchanges.contains("TOOL EXCHANGES THIS TURN"),
            "after the exchanges: {p}"
        );
        assert!(added.starts_with("\n- also check the tests\n"), "{p}");
        assert!(
            added.contains("tool call(s)") || added.contains("LAST tool call"),
            "{p}"
        );
    }
    // The second read offered it again, and again it was taken: each offer
    // joins once, and nothing is announced that the next prompt lacks.
    let twice = prompts[1].matches("- also check the tests").count();
    assert_eq!(twice, 2, "{}", prompts[1]);
    assert_eq!(
        order,
        [
            "steered crew: also check the tests",
            "dial",
            "steered crew: also check the tests",
            "dial"
        ],
        "announced before the dial that carries it"
    );
}

#[test]
fn a_turn_nobody_steered_carries_no_section() {
    let _t = turn();
    let (prompts, order) = run_turn(|| {});
    assert_eq!(prompts.len(), 2);
    assert!(prompts.iter().all(|p| !p.contains(HEAD)), "{prompts:?}");
    assert_eq!(order, ["dial", "dial"]);
}

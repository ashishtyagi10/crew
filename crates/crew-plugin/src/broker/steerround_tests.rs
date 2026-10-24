//! A steer meeting a CLI agent's relay turn on the harness (`hopturn`): taken
//! between rounds, announced before the dial that carries it, and kept in
//! every follow-up for the rest of the turn. The inbox itself is tested in
//! `steer_tests`, whose turn-taking these share.
use std::time::Duration;

use super::tests::{offer, taking, turn, Log};
use super::*;
use crate::broker::adapter::Adapter;
use crate::broker::Broker;
use crate::Registry;

/// An agent that answers from a script, keeps every prompt, and writes each
/// dial to the log.
struct Agent(Mutex<Vec<&'static str>>, Arc<Mutex<Vec<String>>>, Log);

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

/// Run one tool turn on this thread as a task: the follow-up prompts the
/// agent was dialed with, and the log of announcements and dials (the first
/// included), in order.
fn run_turn(during: fn()) -> (Vec<String>, Vec<String>) {
    let log = Log::default();
    let _task = taking(&log);
    let prompts = Arc::new(Mutex::new(Vec::new()));
    let replies = vec![
        "@tool fs:read {\"path\": \"a\"}",
        "@tool fs:read {\"path\": \"b\"}",
        "checked both\n@done",
    ];
    let agent = Agent(Mutex::new(replies), Arc::clone(&prompts), Arc::clone(&log));
    let b = Broker::new(
        Registry::new(vec![Box::new(agent)]),
        6,
        Duration::from_secs(5),
    )
    .with_tools(Arc::new(Tools(during)));
    let emit: Arc<dyn Fn(PluginEvent) + Send + Sync> = Arc::new(|_| {});
    let mut answer = String::new();
    b.run("user", "planner", "task", "t1", &emit, &mut |h| {
        if h.kind == crate::broker::hop::HopKind::Done {
            answer = h.text;
        }
    });
    assert!(answer.contains("checked both"), "{answer}");
    let prompts = prompts.lock().unwrap()[1..].to_vec();
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
            exchanges.contains("TOOL EXCHANGES SO FAR"),
            "after the exchanges: {p}"
        );
        assert!(added.starts_with("\n- also check the tests\n"), "{p}");
        assert!(
            added.contains("round(s) of tool calls") || added.contains("LAST round"),
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
            "dial",
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
    assert_eq!(order, ["dial", "dial", "dial"]);
}

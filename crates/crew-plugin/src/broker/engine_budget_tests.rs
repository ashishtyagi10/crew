use super::*;
use crate::broker::Adapter;
use std::sync::Mutex;
use std::time::Duration;

/// A fixed-reply agent for cost/budget tests.
struct Echo {
    name: String,
    reply: String,
    _n: Mutex<u32>,
}

fn agent(name: &str, reply: &str) -> Box<dyn Adapter> {
    Box::new(Echo {
        name: name.into(),
        reply: reply.into(),
        _n: Mutex::new(0),
    })
}

impl Adapter for Echo {
    fn name(&self) -> &str {
        &self.name
    }
    fn probe(&self) -> bool {
        true
    }
    fn call(&self, _b: &str, _t: Duration) -> Result<String, String> {
        Ok(self.reply.clone())
    }
}

#[test]
fn run_reports_exchanges_and_tokens() {
    let reg = Registry::new(vec![
        agent("claude", "hi\n@next codex"),
        agent("codex", "done\n@done"),
    ]);
    let b = Broker::new(reg, 6, Duration::from_secs(1));
    let stats = b.run("user", "claude", "a task", "t", &mut |_h| {});
    assert_eq!(stats.exchanges, 2);
    assert!(stats.approx_tokens > 0, "token estimate should accrue");
}

#[test]
fn token_budget_terminates_thread() {
    // Agents that keep relaying; a tiny budget must stop the thread early.
    let reg = Registry::new(vec![
        agent("claude", "x\n@next codex"),
        agent("codex", "y\n@next claude"),
    ]);
    let b = Broker::new(reg, 100, Duration::from_secs(1)).with_budget(1);
    let mut hops = Vec::new();
    let stats = b.run("user", "claude", "task", "t", &mut |h| hops.push(h));
    let last = hops.last().unwrap();
    assert_eq!(last.kind, HopKind::Terminated);
    assert!(last.text.contains("token budget"), "{}", last.text);
    assert!(stats.approx_tokens > 1);
}

#[test]
fn self_hand_off_finishes_without_recalling() {
    // An agent that @next's itself must not trigger a redundant self-call.
    let reg = Registry::new(vec![agent("claude", "my take\n@next claude")]);
    let b = Broker::new(reg, 6, Duration::from_secs(1));
    let mut hops = Vec::new();
    let stats = b.run("user", "claude", "task", "t", &mut |h| hops.push(h));
    assert_eq!(stats.exchanges, 1);
    let done = hops.iter().find(|h| h.kind == HopKind::Done).unwrap();
    assert_eq!(done.text, "my take");
}

#[test]
fn repeated_reply_stops_for_no_progress() {
    // Both agents keep emitting the identical body — a stuck loop the no-progress
    // guard must stop well before the (large) hop limit.
    let reg = Registry::new(vec![
        agent("claude", "same\n@next codex"),
        agent("codex", "same\n@next claude"),
    ]);
    let b = Broker::new(reg, 50, Duration::from_secs(1));
    let mut hops = Vec::new();
    b.run("user", "claude", "task", "t", &mut |h| hops.push(h));
    let last = hops.last().unwrap();
    assert_eq!(last.kind, HopKind::Terminated);
    assert!(last.text.contains("no progress"), "{}", last.text);
}

#[test]
fn zero_budget_is_unlimited() {
    let reg = Registry::new(vec![agent("claude", "answer\n@done")]);
    let b = Broker::new(reg, 6, Duration::from_secs(1)); // budget defaults to 0
    let mut hops = Vec::new();
    let stats = b.run("user", "claude", "task", "t", &mut |h| hops.push(h));
    assert_eq!(stats.exchanges, 1);
    assert!(hops.iter().any(|h| h.kind == HopKind::Done));
}

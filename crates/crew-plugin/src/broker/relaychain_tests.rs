//! A turn reads as its answer: one agent alone gets no `turn done` line and
//! no `[done]` head; a real chain of agents keeps its timeline.
use super::*;
use crate::broker::Adapter;
use std::sync::Mutex;

/// Replies in order, repeating the last.
struct Say(&'static str, Vec<&'static str>, Mutex<usize>);

impl Adapter for Say {
    fn name(&self) -> &str {
        self.0
    }
    fn probe(&self) -> bool {
        true
    }
    fn call(&self, _b: &str, _t: std::time::Duration) -> Result<String, String> {
        let mut i = self.2.lock().unwrap();
        let r = self.1.get(*i).or(self.1.last()).unwrap().to_string();
        *i += 1;
        Ok(r)
    }
}

fn turn(agents: Vec<Box<dyn Adapter>>, start: &str) -> Vec<(String, String)> {
    let broker = Broker::new(Registry::new(agents), 6, std::time::Duration::from_secs(1));
    let mut out = Vec::new();
    relay_turn(
        &broker,
        start,
        "task",
        "task",
        "t1",
        &crate::broker::tick::noop_tick_emit(),
        &mut |ev| {
            if let PluginEvent::Message { sender, text, .. } = ev {
                out.push((sender, text));
            }
            Ok(())
        },
    )
    .unwrap();
    out
}

#[test]
fn a_chain_of_two_agents_keeps_its_timeline() {
    let a: Box<dyn Adapter> = Box::new(Say("a", vec!["draft\n@next b"], Mutex::new(0)));
    let b: Box<dyn Adapter> = Box::new(Say("b", vec!["final\n@done"], Mutex::new(0)));
    let msgs = turn(vec![a, b], "a");
    let summary = msgs.iter().find(|(_, t)| t.starts_with("turn done"));
    let (_, s) = summary.unwrap_or_else(|| panic!("a chain is summed up: {msgs:?}"));
    assert!(s.contains("a ") && s.contains("\u{2192} b "), "{s}");
    assert!(
        msgs.iter()
            .any(|(s, t)| s == "b \u{2192} a" && t == "final"),
        "{msgs:?}"
    );
}

#[test]
fn a_bare_done_confirming_a_peer_posts_no_empty_card() {
    let a: Box<dyn Adapter> = Box::new(Say("a", vec!["here it is\n@next b"], Mutex::new(0)));
    let b: Box<dyn Adapter> = Box::new(Say("b", vec!["@done"], Mutex::new(0)));
    let msgs = turn(vec![a, b], "a");
    assert!(
        !msgs
            .iter()
            .any(|(s, t)| s.starts_with("b \u{2192}") && t.trim().is_empty()),
        "{msgs:?}"
    );
    assert!(
        !msgs.iter().any(|(_, t)| t.starts_with("[done]")),
        "{msgs:?}"
    );
}

/// A routed reply runs its one agent alone: a registry of one offers no
/// hand-off, and the prompt says so.
#[test]
fn a_registry_of_one_offers_no_hand_off() {
    let a: Box<dyn Adapter> = Box::new(Say("a", vec!["x"], Mutex::new(0)));
    let b: Box<dyn Adapter> = Box::new(Say("b", vec!["y"], Mutex::new(0)));
    let reg = Registry::new(vec![a, b]).only("b");
    assert_eq!(reg.names(), vec!["b".to_string()]);
    assert!(reg.peers_of("b").is_empty());
    // An unknown name leaves the roster whole rather than empty.
    let c: Box<dyn Adapter> = Box::new(Say("c", vec!["z"], Mutex::new(0)));
    assert_eq!(
        Registry::new(vec![c]).only("nobody").names(),
        vec!["c".to_string()]
    );
}

/// Handing off to a name the roster does not hold finishes the turn with
/// the reply, instead of an "unknown agent" error.
#[test]
fn a_hand_off_to_nobody_is_the_answer() {
    let a: Box<dyn Adapter> = Box::new(Say("a", vec!["here it is\n@next ghost"], Mutex::new(0)));
    let msgs = turn(vec![a], "a");
    assert!(
        msgs.iter()
            .any(|(s, t)| s == "a \u{2192} user" && t == "here it is"),
        "{msgs:?}"
    );
    assert!(
        !msgs.iter().any(|(_, t)| t.contains("unknown agent")),
        "{msgs:?}"
    );
}

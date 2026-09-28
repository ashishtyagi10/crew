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

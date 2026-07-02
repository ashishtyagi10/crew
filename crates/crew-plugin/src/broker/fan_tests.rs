use std::time::Duration;

use super::fan_out;
use crate::{Adapter, PluginEvent, Registry};

/// A fake agent that replies after `delay_ms`, so completion order is testable.
struct Slow(&'static str, u64);
impl Adapter for Slow {
    fn name(&self) -> &str {
        self.0
    }
    fn probe(&self) -> bool {
        true
    }
    fn call(&self, _body: &str, _t: Duration) -> Result<String, String> {
        std::thread::sleep(Duration::from_millis(self.1));
        Ok(format!("{} says hi\n@done", self.0))
    }
}

struct Failing;
impl Adapter for Failing {
    fn name(&self) -> &str {
        "broken"
    }
    fn probe(&self) -> bool {
        true
    }
    fn call(&self, _body: &str, _t: Duration) -> Result<String, String> {
        Err("boom".into())
    }
}

fn run_fan(reg: &Registry, names: &[&str]) -> Vec<PluginEvent> {
    let names: Vec<String> = names.iter().map(|s| s.to_string()).collect();
    let mut evs = Vec::new();
    fan_out(reg, &names, "task", Duration::from_secs(5), &mut |ev| {
        evs.push(ev);
        Ok(())
    })
    .unwrap();
    evs
}

fn messages(evs: &[PluginEvent]) -> Vec<(String, String)> {
    evs.iter()
        .filter_map(|e| match e {
            PluginEvent::Message { sender, text, .. } => Some((sender.clone(), text.clone())),
            _ => None,
        })
        .collect()
}

#[test]
fn replies_stream_in_completion_order_not_roster_order() {
    let reg = Registry::new(vec![
        Box::new(Slow("tortoise", 150)),
        Box::new(Slow("hare", 5)),
    ]);
    let evs = run_fan(&reg, &["tortoise", "hare"]);
    let msgs = messages(&evs);
    // The fast agent's reply lands first even though it was listed second.
    assert!(msgs[0].0.starts_with("hare"), "{msgs:?}");
    assert!(msgs[1].0.starts_with("tortoise"), "{msgs:?}");
    // Control lines are stripped from the replies.
    assert_eq!(msgs[0].1, "hare says hi");
}

#[test]
fn every_agent_thinks_then_goes_idle_and_stats_close_the_turn() {
    let reg = Registry::new(vec![Box::new(Slow("a", 1)), Box::new(Slow("b", 1))]);
    let evs = run_fan(&reg, &["a", "b"]);
    let thinking = evs
        .iter()
        .filter(|e| matches!(e, PluginEvent::Activity { state, .. } if state == "thinking"))
        .count();
    assert_eq!(thinking, 2, "one thinking activity per agent");
    assert!(
        evs.iter()
            .any(|e| matches!(e, PluginEvent::Stats { exchanges: 2, tokens } if *tokens > 0)),
        "{evs:?}"
    );
    // The very last event clears the pane's activity.
    assert!(matches!(
        evs.last().unwrap(),
        PluginEvent::Activity { agent, state, .. } if agent.is_empty() && state == "idle"
    ));
    // The summary names both agents with timings, joined in parallel notation.
    let msgs = messages(&evs);
    let summary = &msgs.last().unwrap().1;
    assert!(summary.contains("fan done"), "{summary}");
    assert!(summary.contains("2 of 2 replied"), "{summary}");
}

#[test]
fn a_failing_agent_reports_but_does_not_sink_the_fan() {
    let reg = Registry::new(vec![Box::new(Slow("ok", 1)), Box::new(Failing)]);
    let evs = run_fan(&reg, &["ok", "broken"]);
    let msgs = messages(&evs);
    assert!(msgs
        .iter()
        .any(|(s, t)| s.starts_with("broken") && t.contains("[error] boom")));
    assert!(msgs.iter().any(|(s, _)| s.starts_with("ok")));
    let summary = &msgs.last().unwrap().1;
    assert!(summary.contains("1 of 2 replied"), "{summary}");
}

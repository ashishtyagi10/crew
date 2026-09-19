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
    fan_out(
        reg,
        &names,
        "task",
        Duration::from_secs(5),
        &crate::broker::tick::noop_tick_emit(),
        &mut |ev| {
            evs.push(ev);
            Ok(())
        },
    )
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
            .any(|e| matches!(e, PluginEvent::Stats { exchanges: 2, tokens, .. } if *tokens > 0)),
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

#[test]
fn a_failing_agent_still_emits_a_zero_usage_per_agent_stat() {
    let reg = Registry::new(vec![Box::new(Slow("ok", 1)), Box::new(Failing)]);
    let evs = run_fan(&reg, &["ok", "broken"]);
    // The failed agent must reconcile the pane's tok display with a zero-usage
    // Stats event of its own — not just the fan-level totals Stats.
    let per_agent = evs.iter().position(|e| {
        matches!(e, PluginEvent::Stats { agent, tokens, exchanges: 0, .. } if agent == "broken" && *tokens == 0)
    });
    assert!(
        per_agent.is_some(),
        "no per-agent Stats for the errored agent: {evs:?}"
    );
    // It must land before the fan-level totals Stats (exchanges == names.len()).
    let totals = evs
        .iter()
        .position(|e| matches!(e, PluginEvent::Stats { exchanges: 2, .. }))
        .expect("fan-level totals Stats missing");
    assert!(
        per_agent.unwrap() < totals,
        "per-agent Stats for the errored agent must precede the fan totals: {evs:?}"
    );
}

/// Every card a fan produces — the replies AND the errors — is marked as one
/// subagent's work, so the host can seat each one as its own section. The
/// turn's own narration (`agent smith`) is not: it is the turn talking.
#[test]
fn every_agents_card_is_marked_as_a_subagent_section() {
    let reg = Registry::new(vec![Box::new(Slow("ok", 1)), Box::new(Failing)]);
    let evs = run_fan(&reg, &["ok", "broken"]);
    let marked: Vec<(String, bool)> = evs
        .iter()
        .filter_map(|e| match e {
            PluginEvent::Message { sender, meta, .. } => Some((
                sender.clone(),
                crate::metatag::has(meta, crate::metatag::SUB),
            )),
            _ => None,
        })
        .collect();
    for (sender, is_sub) in &marked {
        let want = sender != "agent smith";
        assert_eq!(*is_sub, want, "{sender} marked={is_sub}, wanted {want}");
    }
    assert!(marked.iter().any(|(s, m)| s.starts_with("ok") && *m));
    assert!(marked.iter().any(|(s, m)| s.starts_with("broken") && *m));
}

/// The latency survives the mark — `meta` is a tag list, not a flag field.
#[test]
fn the_mark_rides_in_front_of_the_latency() {
    let reg = Registry::new(vec![Box::new(Slow("ok", 1))]);
    let evs = run_fan(&reg, &["ok"]);
    let meta = evs
        .iter()
        .find_map(|e| match e {
            PluginEvent::Message { sender, meta, .. } if sender.starts_with("ok") => {
                Some(meta.clone())
            }
            _ => None,
        })
        .expect("no reply from ok");
    assert!(meta.starts_with("sub \u{00b7} "), "{meta}");
    assert!(meta.ends_with('s'), "the latency is still there: {meta}");
}

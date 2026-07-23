use super::*;
use crate::{CliAdapter, Normalize};

fn reg(names: &[&str]) -> Registry {
    Registry::new(
        names
            .iter()
            .map(|n| {
                Box::new(CliAdapter {
                    name: (*n).into(),
                    program: "true".into(),
                    args: vec![],
                    normalize: Normalize::Raw,
                }) as Box<dyn crate::Adapter>
            })
            .collect(),
    )
}

fn text(ev: &PluginEvent) -> (&str, &str) {
    match ev {
        PluginEvent::Message { sender, text, .. } => (sender, text),
        _ => ("", ""),
    }
}

#[test]
fn split_target_defaults_to_first_agent() {
    let (s, b) = split_target("do the thing", &reg(&["claude", "codex"]));
    assert_eq!((s.as_str(), b.as_str()), ("claude", "do the thing"));
}

#[test]
fn split_target_honours_at_selector() {
    let (s, b) = split_target("@codex review this", &reg(&["claude", "codex"]));
    assert_eq!((s.as_str(), b.as_str()), ("codex", "review this"));
}

#[test]
fn split_target_ignores_unknown_selector() {
    let (s, b) = split_target("@ghost hi", &reg(&["claude"]));
    assert_eq!((s.as_str(), b.as_str()), ("claude", "@ghost hi"));
}

#[test]
fn dialing_becomes_a_thinking_activity() {
    let hop = Hop {
        from: "broker".into(),
        to: "codex".into(),
        hop: 1,
        kind: HopKind::Dialing,
        text: String::new(),
        usage: Default::default(),
    };
    match hop_to_msg(&hop, None) {
        PluginEvent::Activity { agent, state, from } => {
            assert_eq!((agent.as_str(), state.as_str()), ("codex", "thinking"));
            assert_eq!(from, "broker", "activity names who dialed the agent");
        }
        ev => panic!("expected Activity, got {ev:?}"),
    }
}

#[test]
fn reply_hop_is_labelled_from_to() {
    let hop = Hop {
        from: "claude".into(),
        to: "codex".into(),
        hop: 0,
        kind: HopKind::Reply,
        text: "here is my analysis".into(),
        usage: Default::default(),
    };
    let ev = hop_to_msg(&hop, Some(Duration::from_millis(4200)));
    assert_eq!(text(&ev), ("claude → codex", "here is my analysis"));
    match &ev {
        PluginEvent::Message { meta, ts, .. } => {
            assert_eq!(meta, "4.2s");
            assert!(ts.parse::<u64>().is_ok(), "ts should be epoch ms: {ts}");
        }
        _ => panic!("expected Message"),
    }
}

#[test]
fn done_and_error_markers() {
    let mk = |kind, t: &str| Hop {
        from: "a".into(),
        to: "b".into(),
        hop: 0,
        kind,
        text: t.into(),
        usage: Default::default(),
    };
    assert_eq!(text(&hop_to_msg(&mk(HopKind::Done, ""), None)).1, "[done]");
    assert_eq!(
        text(&hop_to_msg(&mk(HopKind::Error, "x"), None)).1,
        "[error] x"
    );
    assert_eq!(
        text(&hop_to_msg(&mk(HopKind::Terminated, "y"), None)).1,
        "[stopped] y"
    );
}

#[test]
fn turn_summary_times_each_agent_in_order() {
    let segs = vec![
        ("planner".to_string(), Duration::from_millis(4200)),
        ("coder".to_string(), Duration::from_millis(8100)),
    ];
    let s = turn_summary(&segs, 2, 950, true);
    assert!(s.contains("planner 4.2s → coder 8.1s"), "{s}");
    assert!(s.contains("2 exchange(s)"), "{s}");
    assert!(s.contains("~950 tok"), "{s}");
}

#[test]
fn turn_summary_without_segments_still_reports_cost() {
    let s = turn_summary(&[], 0, 0, true);
    assert!(s.starts_with("turn done"), "{s}");
    assert!(s.contains("~0 tok"), "{s}");
}

#[test]
fn multi_targets_parses_plus_joined_agents() {
    let r = reg(&["planner", "coder", "reviewer"]);
    let (names, body) = multi_targets("@planner+coder review this", &r).unwrap();
    assert_eq!(names, vec!["planner".to_string(), "coder".to_string()]);
    assert_eq!(body, "review this");
    // Case-insensitive + de-duplicated.
    let (names, _) = multi_targets("@Coder+coder+REVIEWER x", &r).unwrap();
    assert_eq!(names, vec!["coder".to_string(), "reviewer".to_string()]);
}

#[test]
fn multi_targets_rejects_singles_typos_and_plain_tasks() {
    let r = reg(&["planner", "coder"]);
    assert!(
        multi_targets("@planner do it", &r).is_none(),
        "no + selector"
    );
    assert!(
        multi_targets("@planner+ghost do it", &r).is_none(),
        "typo member"
    );
    assert!(
        multi_targets("do it @planner+coder", &r).is_none(),
        "not leading"
    );
    assert!(
        multi_targets("@planner+coder", &r).is_none(),
        "no task body"
    );
}

/// A stub agent that answers `@done ok` and reports real usage, as an
/// API-backed adapter would.
struct UsageAgent;
impl crate::Adapter for UsageAgent {
    fn name(&self) -> &str {
        "planner"
    }
    fn probe(&self) -> bool {
        true
    }
    fn call(&self, _b: &str, _t: std::time::Duration) -> Result<String, String> {
        Ok("@done ok".into())
    }
    fn call_with_usage(
        &self,
        _b: &str,
        _t: std::time::Duration,
    ) -> Result<(String, crate::broker::adapter::Usage), String> {
        Ok((
            "@done ok".into(),
            crate::broker::adapter::Usage {
                input_tokens: 8_192,
                output_tokens: 40,
                cost_microusd: 0,
            },
        ))
    }
}

#[test]
fn relay_streams_live_reply_stats_with_real_usage() {
    let registry = Registry::new(vec![Box::new(UsageAgent)]);
    let broker = Broker::new(registry, 6, std::time::Duration::from_secs(5));
    let mut events = Vec::new();
    relay_turn(
        &broker,
        "planner",
        "task",
        "t1",
        &crate::broker::tick::noop_tick_emit(),
        &mut |ev| {
            events.push(ev);
            Ok(())
        },
    )
    .unwrap();
    // The agent's reply stat streams live with the hop: real spend + context fill.
    let reply_stat = events.iter().position(|e| {
        matches!(e, PluginEvent::Stats { agent, tokens, ctx, .. }
            if agent == "planner" && *tokens == 8_232 && *ctx == 8_192)
    });
    assert!(
        reply_stat.is_some(),
        "live reply stat with usage: {events:?}"
    );
    // The turn total prefers the real count and the summary drops "(approx)".
    let turn_stat = events.iter().position(|e| {
        matches!(e, PluginEvent::Stats { agent, tokens, .. }
            if agent.is_empty() && *tokens == 8_232)
    });
    assert!(turn_stat.is_some(), "real turn total: {events:?}");
    assert!(reply_stat < turn_stat, "reply stat streams before turn end");
    let summary = events.iter().any(|e| {
        matches!(e, PluginEvent::Message { text, .. }
            if text.contains("8232 tok") && !text.contains("approx"))
    });
    assert!(summary, "summary shows real cost: {events:?}");
}

#[test]
fn relay_emits_rate_limited_stats_ticks_between_activity_and_stats() {
    // A real streaming adapter (ApiAdapter over MockProvider), so ticks
    // actually fire mid-hop. MockProvider streams the reply in ~3 chunks,
    // synchronously — a single agent (no peers) skips the protocol-repair
    // path, so the hop is exactly one `call_with_usage_ticked` call.
    let provider: std::sync::Arc<dyn crew_hive::Provider> =
        std::sync::Arc::new(crew_hive::MockProvider {
            reply: "one two three four five six seven eight\n@done".into(),
        });
    let agent =
        crate::broker::apiadapter::ApiAdapter::new("planner", "m", "", None, provider).unwrap();
    let registry = Registry::new(vec![Box::new(agent)]);
    let broker = Broker::new(registry, 6, std::time::Duration::from_secs(5));

    // Both `emit` and `tick_emit` push into the SAME ordered log — ticks and
    // hop events come from two different closures, but the call is single-
    // threaded (ApiAdapter blocks a current-thread tokio runtime), so a shared
    // Vec behind one Mutex preserves the true temporal order.
    let events: std::sync::Arc<std::sync::Mutex<Vec<PluginEvent>>> = Default::default();
    let tick_sink = events.clone();
    let tick_emit: std::sync::Arc<dyn Fn(PluginEvent) + Send + Sync> =
        std::sync::Arc::new(move |ev| tick_sink.lock().unwrap().push(ev));
    let emit_sink = events.clone();
    relay_turn(
        &broker,
        "planner",
        "task",
        "t1",
        &tick_emit,
        &mut move |ev| {
            emit_sink.lock().unwrap().push(ev);
            Ok(())
        },
    )
    .unwrap();
    let events = events.lock().unwrap().clone();

    let idx = |pred: &dyn Fn(&PluginEvent) -> bool| events.iter().position(pred);
    let thinking =
        idx(&|e| matches!(e, PluginEvent::Activity { state, .. } if state == "thinking"))
            .expect("a thinking activity");
    // The mock streams synchronously, so all chunks may land within 1ms — the
    // 150ms gate then allows only the FIRST tick. That is spec-correct
    // behavior, so assert on presence/ordering/monotonicity, not an exact count.
    let first_tick = idx(&|e| matches!(e, PluginEvent::StatsTick { .. }))
        .expect("at least one tick between the dial and the hop's Stats");
    let stats = idx(&|e| matches!(e, PluginEvent::Stats { agent, .. } if !agent.is_empty()))
        .expect("a per-agent Stats event");
    assert!(
        thinking < first_tick && first_tick < stats,
        "tick lands mid-hop: {events:?}"
    );

    let ticks: Vec<u64> = events
        .iter()
        .filter_map(|e| match e {
            PluginEvent::StatsTick { tokens, .. } => Some(*tokens),
            _ => None,
        })
        .collect();
    assert!(!ticks.is_empty());
    assert!(
        ticks.windows(2).all(|w| w[0] < w[1]),
        "ticks only fire when the estimate grew: {ticks:?}"
    );
}

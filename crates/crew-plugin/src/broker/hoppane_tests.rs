use std::sync::{Arc, Mutex};

use crew_hive::harness::Events;
use crew_hive::HiveEvent;

/// The tool line keeps a long result's head for its click-open view, and
/// says it was cut; the model still reads the result as the harness clips it.
#[test]
fn the_tool_line_keeps_a_long_results_head() {
    let long = format!("{}TAIL", "y".repeat(3_000));
    assert_eq!(super::kept("short\n"), "short");
    let kept = super::kept(&long);
    assert!(
        kept.starts_with("yyy") && kept.ends_with("\n\u{2026}"),
        "{kept}"
    );
    assert_eq!(kept.chars().count(), 2_000 + 2);
    assert!(!kept.contains("TAIL"));
}

/// The hop's token estimate only grows, whatever each call of the turn
/// streams: a short follow-up after a long first reply still reads past it,
/// so the tick gate, which emits only on growth, never swallows it.
#[test]
fn the_token_estimate_only_grows_across_calls() {
    let seen: Arc<Mutex<Vec<u64>>> = Arc::default();
    let keep = Arc::clone(&seen);
    let mut stream = crate::broker::adapter::HopStream::noop();
    stream.on_tokens = Arc::new(move |n| keep.lock().unwrap().push(n));
    let pane = super::Pane {
        stream,
        tally: Arc::default(),
    };
    let delta = |text: &str| HiveEvent::OutputDelta {
        agent: crew_hive::AgentId(1),
        text: text.into(),
    };
    pane.publish(delta(&"x".repeat(800))); // the first call's long reply
    pane.publish(delta("short")); // the follow-up's
    pane.publish(delta(" answer"));
    let seen = seen.lock().unwrap().clone();
    assert_eq!(seen, [200, 201, 203]);
}

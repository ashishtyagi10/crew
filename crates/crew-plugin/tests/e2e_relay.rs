//! End-to-end smoke tests of the relay through the real `crew-broker-plugin`
//! binary, using the inbuilt agents backed by the `CREW_BROKER_MOCK_REPLY`
//! fixed-reply mock (no network). The multi-hop relay logic itself — A→B, B→A,
//! the 3-way relay, the loop guard — is covered exhaustively by the engine unit
//! tests (`broker::engine::tests`); here we prove the real binary streams the
//! protocol end to end and surfaces the cost summary.
mod common;
use common::{has_leg, messages, run_broker, unique_dir, PluginEvent};

const SEND: &str = r#"{"type":"send","channel":"crew","text":"do it"}"#;

#[test]
fn relay_runs_through_the_binary_and_finishes() {
    let dir = unique_dir("relay-done");
    let mock = ("CREW_BROKER_MOCK_REPLY", "did the work\n@done");
    let ev = run_broker(&dir, &[mock], &[SEND]);
    let msgs = messages(&ev);
    // The default starting agent (planner) ran and finished back to the user.
    assert!(has_leg(&ev, "planner → user"), "{msgs:?}");
    // The done leg carries the answer with the control line stripped.
    assert!(
        msgs.iter()
            .any(|(s, t)| s == "planner → user" && t.contains("did the work")),
        "{msgs:?}"
    );
    // A cost summary is surfaced at the end.
    assert!(
        msgs.iter()
            .any(|(s, t)| s == "crew" && t.contains("tokens")),
        "{msgs:?}"
    );
}

#[test]
fn dialing_is_streamed_as_a_live_activity() {
    let dir = unique_dir("relay-stream");
    let mock = ("CREW_BROKER_MOCK_REPLY", "ok\n@done");
    let ev = run_broker(&dir, &[mock], &[SEND]);
    // The broker streams a thinking activity as it dials the agent…
    assert!(
        ev.iter().any(|e| matches!(
            e,
            PluginEvent::Activity { agent, state } if agent == "planner" && state == "thinking"
        )),
        "{ev:?}"
    );
    // …and clears it when the turn ends.
    assert!(
        ev.iter().any(|e| matches!(
            e,
            PluginEvent::Activity { agent, state } if agent.is_empty() && state == "idle"
        )),
        "{ev:?}"
    );
}

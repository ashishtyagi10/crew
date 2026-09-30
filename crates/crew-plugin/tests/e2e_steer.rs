//! A `steer` through the real `crew-broker-plugin` binary. What a steer DOES
//! — join the running task's next tool round — is covered by the unit tests
//! beside `broker::steer`, which can hold a round open; here we prove the
//! binary reads the line and that a steer is only ever an OFFER: it never
//! starts a task of its own, and one nothing took is never announced, so the
//! pane's queued copy is what gets sent.
mod common;
use common::{messages, run_broker, seed_specialists, unique_dir, PluginEvent};

const STEER: &str = r#"{"type":"steer","channel":"crew","text":"also check the tests"}"#;

fn steered(ev: &[PluginEvent]) -> usize {
    ev.iter()
        .filter(|e| matches!(e, PluginEvent::Steered { .. }))
        .count()
}

fn task_starts(ev: &[PluginEvent]) -> usize {
    ev.iter()
        .filter(|e| matches!(e, PluginEvent::Task { running: true, .. }))
        .count()
}

#[test]
fn a_steer_with_nothing_running_is_ignored_and_the_broker_keeps_serving() {
    let dir = unique_dir("steer-idle");
    let stop = r#"{"type":"send","channel":"crew","text":"/stop"}"#;
    let ev = run_broker(&dir, &[], &[STEER, stop]);
    assert_eq!(steered(&ev), 0, "{ev:?}");
    assert_eq!(task_starts(&ev), 0, "a steer is not a task: {ev:?}");
    let msgs = messages(&ev);
    assert!(
        msgs.iter()
            .any(|(s, t)| s == "agent smith" && t.contains("nothing is running")),
        "the line after it was still served: {msgs:?}"
    );
}

#[test]
fn a_steer_no_round_takes_is_never_announced() {
    let dir = unique_dir("steer-untaken");
    seed_specialists(&dir, &["planner"]);
    // A reply with no tool call: the task has no round for a steer to join.
    let mock = ("CREW_BROKER_MOCK_REPLY", "did it\n@done");
    let send = r#"{"type":"send","channel":"crew","text":"@planner do it"}"#;
    let ev = run_broker(&dir, &[mock], &[send, STEER]);
    assert_eq!(steered(&ev), 0, "{ev:?}");
    assert_eq!(task_starts(&ev), 1, "only the Send started one: {ev:?}");
}

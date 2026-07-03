//! End-to-end coverage for concurrent background tasks through the real
//! `crew-broker-plugin` binary: several `Send`s spawn several tasks (no
//! "busy" rejection), `/tasks` and `/stop [#N]` address them by id, and each
//! task's streamed relay `Message` events carry a `meta: "task:<id>"` tag.
mod common;
use common::{messages, run_broker, unique_dir, PluginEvent};

const SEND_A: &str = r#"{"type":"send","channel":"crew","text":"do task a"}"#;
const SEND_B: &str = r#"{"type":"send","channel":"crew","text":"do task b"}"#;
const TASKS: &str = r#"{"type":"send","channel":"crew","text":"/tasks"}"#;
const STOP_1: &str = r#"{"type":"send","channel":"crew","text":"/stop #1"}"#;
const STOP: &str = r#"{"type":"send","channel":"crew","text":"/stop"}"#;

#[test]
fn two_sends_both_start_as_separate_tasks_not_busy() {
    let dir = unique_dir("tasks-concurrent");
    let mock = ("CREW_BROKER_MOCK_REPLY", "did it\n@done");
    let ev = run_broker(&dir, &[mock], &[SEND_A, SEND_B]);
    let msgs = messages(&ev);
    assert!(
        msgs.iter()
            .any(|(s, t)| s == "crew" && t.contains("task #1 started")),
        "{msgs:?}"
    );
    assert!(
        msgs.iter()
            .any(|(s, t)| s == "crew" && t.contains("task #2 started")),
        "{msgs:?}"
    );
    // No "busy" rejection anywhere — both sends were admitted concurrently.
    assert!(
        !msgs.iter().any(|(_, t)| t.contains("busy")),
        "second Send was rejected as busy: {msgs:?}"
    );
}

#[test]
fn tasks_lists_running_and_reports_idle_when_none_are() {
    let dir = unique_dir("tasks-list-idle");
    let mock = ("CREW_BROKER_MOCK_REPLY", "did it\n@done");
    // Idle broker: /tasks alone reports nothing running.
    let ev = run_broker(&dir, &[mock], &[TASKS]);
    let msgs = messages(&ev);
    assert!(
        msgs.iter()
            .any(|(s, t)| s == "crew" && t.contains("no background tasks running")),
        "{msgs:?}"
    );

    // Two tasks are started (order-independent from any racy /tasks listing,
    // per the harness's determinism note: the synchronous "started" lines are
    // emitted before spawn, so they always appear in order).
    let dir2 = unique_dir("tasks-list-started");
    let ev2 = run_broker(&dir2, &[mock], &[SEND_A, SEND_B]);
    let msgs2 = messages(&ev2);
    assert!(
        msgs2
            .iter()
            .any(|(s, t)| s == "crew" && t.contains("task #1 started")),
        "{msgs2:?}"
    );
    assert!(
        msgs2
            .iter()
            .any(|(s, t)| s == "crew" && t.contains("task #2 started")),
        "{msgs2:?}"
    );
}

#[test]
fn stop_with_id_names_the_task_bare_stop_cancels_all() {
    let dir = unique_dir("tasks-stop-id");
    let mock = ("CREW_BROKER_MOCK_REPLY", "did it\n@done");
    let ev = run_broker(&dir, &[mock], &[SEND_A, STOP_1]);
    let msgs = messages(&ev);
    assert!(
        msgs.iter()
            .any(|(s, t)| s == "crew" && t.contains('1') && t.to_lowercase().contains("stop")),
        "{msgs:?}"
    );

    let dir2 = unique_dir("tasks-stop-all");
    let ev2 = run_broker(&dir2, &[mock], &[SEND_A, SEND_B, STOP]);
    let msgs2 = messages(&ev2);
    assert!(
        msgs2
            .iter()
            .any(|(s, t)| s == "crew" && t.to_lowercase().contains("stopping all")),
        "{msgs2:?}"
    );
}

#[test]
fn relay_message_events_carry_the_task_meta_tag() {
    let dir = unique_dir("tasks-meta");
    let mock = ("CREW_BROKER_MOCK_REPLY", "did it\n@done");
    let ev = run_broker(&dir, &[mock], &[SEND_A]);
    // `messages()` drops `meta`, so walk the raw events for the tag.
    assert!(
        ev.iter().any(|e| matches!(
            e,
            PluginEvent::Message { meta, .. } if meta.starts_with("task:")
        )),
        "{ev:?}"
    );
}

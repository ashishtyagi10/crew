//! End-to-end coverage for concurrent background tasks through the real
//! `crew-broker-plugin` binary: several `Send`s spawn several tasks (no
//! "busy" rejection), `/tasks` and `/stop [#N]` address them by id, and each
//! task's streamed relay `Message` events carry a `meta: "task:<id>"` tag.
mod common;
use common::{messages, run_broker, unique_dir, PluginEvent};

const SEND_A: &str = r#"{"type":"send","channel":"crew","text":"do task a"}"#;
const SEND_B: &str = r#"{"type":"send","channel":"crew","text":"do task b"}"#;
const TASKS: &str = r#"{"type":"send","channel":"crew","text":"/tasks"}"#;
const STOP: &str = r#"{"type":"send","channel":"crew","text":"/stop"}"#;
const STOP_999: &str = r#"{"type":"send","channel":"crew","text":"/stop #999"}"#;

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
    // Live task-pool capacity (`n/max`) is stamped on each start line and
    // increments as tasks are admitted.
    assert!(
        msgs.iter()
            .any(|(s, t)| s == "crew" && t.contains("task #1 started") && t.contains("1/")),
        "first start line missing 1/max capacity: {msgs:?}"
    );
    assert!(
        msgs.iter()
            .any(|(s, t)| s == "crew" && t.contains("task #2 started") && t.contains("2/")),
        "second start line missing 2/max capacity: {msgs:?}"
    );
    // No "busy" rejection anywhere — both sends were admitted concurrently.
    assert!(
        !msgs.iter().any(|(_, t)| t.contains("busy")),
        "second Send was rejected as busy: {msgs:?}"
    );
    // Both tasks actually RAN to completion (not just admitted): each emits its
    // own `done` line and its own agent reply. `send` spawns each worker without
    // joining the previous handle (see stdio.rs), so the two run concurrently;
    // the harness joins both on EOF, so both must have finished before exit.
    assert!(
        msgs.iter()
            .any(|(s, t)| s == "crew" && t.contains("task #1 done")),
        "task #1 never completed: {msgs:?}"
    );
    assert!(
        msgs.iter()
            .any(|(s, t)| s == "crew" && t.contains("task #2 done")),
        "task #2 never completed: {msgs:?}"
    );
    // Two independent agent replies landed — one per task — proving both tasks
    // did real work, not that one was dropped or serialized out.
    let replies = msgs
        .iter()
        .filter(|(s, t)| s.contains(" \u{2192} ") && t.contains("did it"))
        .count();
    assert!(
        replies >= 2,
        "expected a reply from each task, got {replies}: {msgs:?}"
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

// Coverage boundary: an e2e can't deterministically cancel a RUNNING mock task
// here — the mock provider replies and `@done`s instantly, so the task is reaped
// before the next queued stdin line (`/stop`) is even read, making any
// "stopping task #N…" / "stopping all N…" assertion race the completion. The
// live-cancellation semantics (trip one flag vs. all) are locked in
// deterministically by the Task-1 unit tests (`cancel_trips_only_that_task`,
// `admit_is_false_once_at_the_cap`) in `broker/tasks_tests.rs`. What we CAN pin
// end-to-end is the two timing-independent /stop replies below.
#[test]
fn stop_reports_unknown_id_and_idle_deterministically() {
    // `/stop #999` for an id that never existed → always "no task #999",
    // regardless of whether the prior Send has finished.
    let dir = unique_dir("tasks-stop-unknown");
    let mock = ("CREW_BROKER_MOCK_REPLY", "did it\n@done");
    let ev = run_broker(&dir, &[mock], &[SEND_A, STOP_999]);
    let msgs = messages(&ev);
    assert!(
        msgs.iter()
            .any(|(s, t)| s == "crew" && t.contains("no task #999")),
        "{msgs:?}"
    );

    // Bare `/stop` on an idle broker (no prior Send) → always "nothing is
    // running": there is nothing to race.
    let dir2 = unique_dir("tasks-stop-idle");
    let ev2 = run_broker(&dir2, &[mock], &[STOP]);
    let msgs2 = messages(&ev2);
    assert!(
        msgs2
            .iter()
            .any(|(s, t)| s == "crew" && t.contains("nothing is running")),
        "{msgs2:?}"
    );
}

#[test]
fn relay_message_events_carry_the_task_meta_tag() {
    let dir = unique_dir("tasks-meta");
    let mock = ("CREW_BROKER_MOCK_REPLY", "did it\n@done");
    let ev = run_broker(&dir, &[mock], &[SEND_A]);
    // `messages()` drops `meta`, so walk the raw events for the tag. Assert the
    // ACTUAL AGENT REPLY carries the tag — its `sender` is a relay leg label
    // containing the `→` (e.g. `"planner → user"`), and its `meta` already holds
    // the hop latency, so a naive `if meta.is_empty()` guard would ship it
    // untagged. A weaker `any(meta.starts_with("task:"))` passes on an untagged
    // bookkeeping line, so it must pin the reply specifically.
    assert!(
        ev.iter().any(|e| matches!(
            e,
            PluginEvent::Message { sender, meta, .. }
                if sender.contains(" \u{2192} ") && meta.starts_with("task:")
        )),
        "the agent reply must be tagged task:<id>, got {ev:?}"
    );
}

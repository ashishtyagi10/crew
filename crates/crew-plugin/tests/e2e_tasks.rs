//! End-to-end coverage for concurrent background tasks through the real
//! `crew-broker-plugin` binary: several `Send`s spawn several tasks (no
//! "busy" rejection), each announces its start and end via `PluginEvent::Task`
//! (what the pane footer shows, and what replaced `/tasks`), `/stop [#N]`
//! addresses them by id, and each task's streamed relay `Message` events carry
//! a `meta: "task:<id>"` tag.
mod common;
use common::{messages, run_broker, seed_specialists, unique_dir, PluginEvent};

// `@`-addressed so they route to the relay (these pin the task-pool machinery
// and relay-leg `task:<id>` meta tagging; a plain message is now the default
// swarm, whose replies aren't relay legs). `planner` must be seeded into each
// test's isolated store first — see `seed_specialists` (there is no inbuilt
// roster).
const SEND_A: &str = r#"{"type":"send","channel":"crew","text":"@planner do task a"}"#;
const SEND_B: &str = r#"{"type":"send","channel":"crew","text":"@planner do task b"}"#;
const STOP: &str = r#"{"type":"send","channel":"crew","text":"/stop"}"#;
const STOP_999: &str = r#"{"type":"send","channel":"crew","text":"/stop #999"}"#;

#[test]
fn two_sends_both_run_as_separate_tasks_not_busy() {
    let dir = unique_dir("tasks-concurrent");
    seed_specialists(&dir, &["planner"]);
    let mock = ("CREW_BROKER_MOCK_REPLY", "did it\n@done");
    let ev = run_broker(&dir, &[mock], &[SEND_A, SEND_B]);
    let msgs = messages(&ev);
    // No "busy" rejection anywhere — both sends were admitted concurrently.
    // (The old "task #N started/done" status lines were retired for a cleaner,
    // Opencode-style stream; the agent replies themselves are now the proof a
    // task ran.)
    assert!(
        !msgs.iter().any(|(_, t)| t.contains("busy")),
        "second Send was rejected as busy: {msgs:?}"
    );
    // Two independent agent replies landed — one per task — proving both tasks
    // did real work, not that one was dropped or serialized out. `send` spawns
    // each worker without joining the previous handle (see stdio.rs), so the
    // two run concurrently; the harness joins both on EOF, so both must have
    // finished before exit.
    let replies = msgs
        .iter()
        .filter(|(s, t)| s.contains(" \u{2192} ") && t.contains("did it"))
        .count();
    assert!(
        replies >= 2,
        "expected a reply from each task, got {replies}: {msgs:?}"
    );
}

/// `/tasks` is gone; `PluginEvent::Task` replaced it, and this is the contract
/// the pane's footer depends on: every task announces its START from the stdin
/// loop and its END from the worker itself. The end is the half that could not
/// exist before — `Tasks::reap` runs lazily on the next command, so a task
/// finishing was not an observable moment anywhere.
#[test]
fn every_task_announces_its_start_and_its_end() {
    let dir = unique_dir("tasks-lifecycle");
    seed_specialists(&dir, &["planner"]);
    let mock = ("CREW_BROKER_MOCK_REPLY", "did it\n@done");
    let ev = run_broker(&dir, &[mock], &[SEND_A, SEND_B]);
    let lifecycle: Vec<(u64, bool)> = ev
        .iter()
        .filter_map(|e| match e {
            PluginEvent::Task { id, running, .. } => Some((*id, *running)),
            _ => None,
        })
        .collect();
    // Two tasks, each seen starting and ending. Ids are per-broker monotonic.
    for id in [1u64, 2] {
        assert!(
            lifecycle.contains(&(id, true)),
            "task #{id} never announced a start: {lifecycle:?}"
        );
        assert!(
            lifecycle.contains(&(id, false)),
            "task #{id} never announced an end: {lifecycle:?}"
        );
    }
    // The start carries the label the footer would show; the end does not
    // repeat it (the host already has it).
    let labelled = ev.iter().any(|e| {
        matches!(e, PluginEvent::Task { label, running: true, .. } if label.contains("do task a"))
    });
    assert!(labelled, "no start event carried its label: {ev:?}");
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
    seed_specialists(&dir, &["planner"]);
    let mock = ("CREW_BROKER_MOCK_REPLY", "did it\n@done");
    let ev = run_broker(&dir, &[mock], &[SEND_A, STOP_999]);
    let msgs = messages(&ev);
    assert!(
        msgs.iter()
            .any(|(s, t)| s == "agent smith" && t.contains("no task #999")),
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
            .any(|(s, t)| s == "agent smith" && t.contains("nothing is running")),
        "{msgs2:?}"
    );
}

#[test]
fn relay_message_events_carry_the_task_meta_tag() {
    let dir = unique_dir("tasks-meta");
    seed_specialists(&dir, &["planner"]);
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

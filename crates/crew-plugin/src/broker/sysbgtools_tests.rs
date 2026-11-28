//! The tools as the agent calls them, through `call`.
use super::*;

fn go(bg: &SysBg, tool: &str, args: serde_json::Value) -> Result<String, String> {
    call(bg, tool, &args.to_string())
}

#[test]
fn spawn_waits_for_the_line_it_was_told_to() {
    let bg = SysBg::default();
    let out = go(
        &bg,
        "spawn",
        serde_json::json!({"cmd": "echo starting; sleep 0.3; echo listening on 8080; sleep 300", "wait_for": "listening", "timeout_ms": 10000}),
    )
    .unwrap();
    assert!(out.starts_with("{\"id\": 1}"), "{out}");
    assert!(
        out.contains("saw \u{201c}listening\u{201d}") && out.contains("listening on 8080"),
        "{out}"
    );
    assert!(out.contains("running"), "{out}");
    let late = go(&bg, "kill", serde_json::json!({"id": 1})).unwrap();
    assert!(late.starts_with("#1 "), "{late}");
}

#[test]
fn spawn_says_so_when_the_text_never_comes() {
    let bg = SysBg::default();
    let out = go(
        &bg,
        "spawn",
        serde_json::json!({"cmd": "sleep 300", "wait_for": "ready", "timeout_ms": 300}),
    )
    .unwrap();
    assert!(out.contains("not seen"), "{out}");
    let out = go(
        &bg,
        "spawn",
        serde_json::json!({"cmd": "echo nope; exit 1", "wait_for": "ready"}),
    )
    .unwrap();
    assert!(
        out.contains("exited while waiting") && out.contains("exited with status 1"),
        "{out}"
    );
}

#[test]
fn logs_tail_the_latest_lines_and_wait_for_more() {
    let bg = SysBg::default();
    go(&bg, "spawn", serde_json::json!({"cmd": "for i in 1 2 3 4 5; do echo line$i; done; sleep 0.3; echo late; sleep 300"})).unwrap();
    let out = go(
        &bg,
        "logs",
        serde_json::json!({"id": 1, "tail": 2, "wait_for": "late", "timeout_ms": 10000}),
    )
    .unwrap();
    assert!(
        out.contains("line5") && out.contains("late") && !out.contains("line4"),
        "{out}"
    );
    assert!(out.contains("running"), "{out}");
}

#[test]
fn another_tasks_id_is_unknown() {
    let (mine, theirs) = (SysBg::default(), SysBg::default());
    go(&theirs, "spawn", serde_json::json!({"cmd": "sleep 300"})).unwrap();
    for tool in ["logs", "kill"] {
        let why = go(&mine, tool, serde_json::json!({"id": 1})).unwrap_err();
        assert!(why.contains("no background process #1"), "{why}");
    }
}

/// Through the session's own surface: `logs` is a read, but its answer changes between two asks,
/// so the loop must never replay the first; and the task's `bg` is what its end reaps.
#[test]
fn the_session_surface_never_replays_logs_and_the_task_owns_the_process() {
    use crate::broker::session::Session;
    use std::sync::{atomic::AtomicBool, Arc};
    let task = Session::new().snapshot_with_cancel(Arc::new(AtomicBool::new(false)));
    let hop = task.tools_with_sys(true).unwrap();
    assert!(!hop.repeatable("sys", "logs") && !hop.repeatable("sys", "kill"));
    assert!(hop.repeatable("sys", "read_file"));
    let out = hop
        .call(
            "sys",
            "spawn",
            r#"{"cmd": "echo up; sleep 300", "wait_for": "up"}"#,
        )
        .unwrap();
    assert!(out.contains("saw"), "{out}");
    assert!(task.bg.get(1).is_ok(), "the task's own registry holds it");
    task.bg.reap();
    assert!(task.bg.get(1).is_err());
}

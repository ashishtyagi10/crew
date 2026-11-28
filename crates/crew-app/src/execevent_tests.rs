use super::*;

fn msg(sender: &str, text: &str, meta: &str) -> PluginEvent {
    PluginEvent::Message {
        channel: "crew".into(),
        sender: sender.into(),
        text: text.into(),
        ts: String::new(),
        meta: meta.into(),
    }
}
fn task(id: u64, running: bool) -> PluginEvent {
    PluginEvent::Task {
        id,
        label: String::new(),
        running,
        seq: 1,
    }
}
fn approval() -> PluginEvent {
    PluginEvent::Approval {
        id: "a1".into(),
        tool: "sys:shell".into(),
        tier: "irreversible".into(),
        reply_to: "user".into(),
        question: "run rm -rf x?\nmore".into(),
    }
}

#[test]
fn the_last_task_reply_is_the_answer_and_a_clean_run_exits_0() {
    let mut run = Run::new(false);
    run.absorb(task(1, true));
    run.absorb(msg("leaf-0", "draft", "task:1"));
    run.absorb(msg("merge", "the answer", "task:1"));
    assert!(!run.done);
    run.absorb(task(1, false));
    assert!(run.done);
    assert_eq!(run.answer.as_deref(), Some("the answer"));
    assert_eq!(run.exit_code(), EXIT_OK);
}

#[test]
fn smith_notes_are_progress_never_the_answer() {
    let mut run = Run::new(false);
    let r = run.absorb(msg("agent smith", "planned 3 tasks", "task:1"));
    assert_eq!(r.log, vec!["planned 3 tasks"]);
    assert!(run.answer.is_none());
    assert_eq!(
        run.exit_code(),
        EXIT_FAILED,
        "no answer at all is a failure"
    );
    // The banner (no task meta) is ignored outright.
    assert!(run.absorb(msg("agent smith", "banner", "")).log.is_empty());
}

#[test]
fn an_approval_is_denied_by_default_and_exits_3() {
    let mut run = Run::new(false);
    let r = run.absorb(approval());
    assert!(matches!(
        r.reply,
        Some(PluginCommand::Approve {
            granted: false,
            hunks: None,
            ..
        })
    ));
    run.absorb(msg("merge", "done anyway", "task:1"));
    assert_eq!(run.exit_code(), EXIT_REFUSED);
}

#[test]
fn yes_allows_the_approval_and_the_run_can_still_exit_0() {
    let mut run = Run::new(true);
    let r = run.absorb(approval());
    assert!(matches!(
        r.reply,
        Some(PluginCommand::Approve { granted: true, .. })
    ));
    run.absorb(msg("merge", "ok", "task:1"));
    assert_eq!(run.exit_code(), EXIT_OK);
}

#[test]
fn an_error_event_fails_the_run_even_with_an_answer() {
    let mut run = Run::new(false);
    run.absorb(msg("merge", "partial", "task:1"));
    run.absorb(PluginEvent::Error {
        message: "provider 500".into(),
    });
    assert_eq!(run.exit_code(), EXIT_FAILED);
}

#[test]
fn another_tasks_end_does_not_end_ours() {
    let mut run = Run::new(false);
    run.absorb(task(2, true));
    run.absorb(task(1, false));
    assert!(!run.done);
}

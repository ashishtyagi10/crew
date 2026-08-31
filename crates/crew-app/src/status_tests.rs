use crate::app::CrewApp;

#[test]
fn log_entry_is_timestamped_but_flash_is_not() {
    let mut app = CrewApp::default();
    app.set_status("hello world");
    // The input-bar flash is the bare message…
    assert_eq!(app.active_status(), Some("hello world"));
    // …while the LOG entry carries an `HH:MM` stamp before it.
    let last = app.log.last().expect("log has the entry");
    assert!(last.text.ends_with("hello world"));
    assert!(last.text.contains(':') && last.text != "hello world");
    assert_eq!(last.level, crate::applog::LogLevel::Info);
}

#[test]
fn error_status_flags_its_log_entry() {
    let mut app = CrewApp::default();
    app.set_status_err("broker fell over");
    // Same flash as an info status…
    assert_eq!(app.active_status(), Some("broker fell over"));
    // …but the LOG entry carries the error level for the renderer.
    assert_eq!(
        app.log.last().unwrap().level,
        crate::applog::LogLevel::Error
    );
}

#[test]
fn clear_log_empties_then_notes_the_reset() {
    let mut app = CrewApp::default();
    app.set_status("a");
    app.set_status("b");
    assert_eq!(app.log.len(), 2);
    app.clear_log();
    // Cleared down to just the single "cleared" note (not blank).
    assert_eq!(app.log.len(), 1);
    assert!(app.log[0].text.ends_with("activity log cleared"));
}

#[test]
fn notify_logs_a_flash_when_enabled() {
    use crate::notify::NotifyKind;
    let mut app = CrewApp::default();
    app.notify(NotifyKind::AgentDone, "crew".into(), "claude".into());
    assert_eq!(app.active_status(), Some("✓ claude finished in crew"));
    assert!(app
        .log
        .last()
        .unwrap()
        .text
        .contains("claude finished in crew"));
}

/// "It is done" and "it went wrong" are not the same news, and only one
/// of them is worth getting up for: a failure is an ALERT toast in the
/// bell colour, legended `failed`.
#[test]
fn a_failed_command_is_told_apart_from_a_finished_one() {
    use crate::notify::NotifyKind;
    let mut app = CrewApp::default();
    app.notify(
        NotifyKind::Failed,
        "crew".into(),
        "cargo test (2m14) \u{2014} exit 101".into(),
    );
    let said = app.active_status().unwrap_or_default().to_string();
    assert!(said.starts_with('\u{2717}'), "{said:?}");
    assert!(said.contains("exit 101"), "the status is in it: {said:?}");
    assert!(said.contains("failed in crew"), "{said:?}");
    // …and the card is an ALERT: the bell stroke, legended `failed`. A
    // failure drawn as quietly as a success is a failure you scroll past.
    assert_eq!(app.toasts.newest(), Some(("failed", true)));

    let mut ok = CrewApp::default();
    ok.notify(NotifyKind::AgentDone, "crew".into(), "cargo test".into());
    assert_eq!(ok.toasts.newest(), Some(("done", false)));
}

/// One switch for both. Turning "a command finished" off must silence the
/// failure too — splitting the preference in two would ask the user to
/// say twice that they want to hear about commands finishing.
#[test]
fn one_switch_governs_both_outcomes() {
    use crate::notify::NotifyKind;
    let mut app = CrewApp::default();
    app.config.notify_agent_done = false;
    for kind in [NotifyKind::AgentDone, NotifyKind::Failed] {
        app.notify(kind, "crew".into(), "cargo test".into());
        assert_eq!(app.active_status(), None, "{kind:?} spoke anyway");
    }
}

#[test]
fn notify_respects_the_per_kind_toggle() {
    use crate::notify::NotifyKind;
    let mut app = CrewApp::default();
    app.config.notify_bell = false;
    let before = app.log.len();
    app.notify(NotifyKind::Bell, "crew".into(), String::new());
    assert_eq!(app.log.len(), before, "bell notifications are disabled");
}

#[test]
fn notify_master_switch_suppresses_everything() {
    use crate::notify::NotifyKind;
    let mut app = CrewApp::default();
    app.config.notify = false;
    let before = app.log.len();
    app.notify(NotifyKind::Exited, "crew".into(), String::new());
    assert_eq!(
        app.log.len(),
        before,
        "master switch off → no notifications"
    );
}

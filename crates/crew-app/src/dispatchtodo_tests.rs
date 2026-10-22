use crate::app::CrewApp;

#[test]
fn notify_off_then_on_toggles_the_master_switch() {
    let mut app = CrewApp::default();
    assert!(app.config.notify);
    app.notify_command("off");
    assert!(!app.config.notify);
    app.notify_command("on");
    assert!(app.config.notify);
}

#[test]
fn notify_add_appends_a_pattern_then_clear_empties() {
    let mut app = CrewApp::default();
    app.notify_command("add error");
    assert_eq!(app.config.notify_patterns, vec!["error".to_string()]);
    app.notify_command("clear");
    assert!(app.config.notify_patterns.is_empty());
}

#[test]
fn notify_add_without_text_adds_nothing() {
    let mut app = CrewApp::default();
    app.notify_command("add    ");
    assert!(app.config.notify_patterns.is_empty());
}

#[test]
fn todo_reminders_turns_the_sync_on_and_off_and_keeps_the_switch() {
    let mut app = CrewApp::default();
    app.todo_command("reminders");
    assert_eq!(app.config.todo_reminders, cfg!(target_os = "macos"));
    app.todo_command("reminders off");
    assert!(!app.config.todo_reminders);
    app.todo_command("reminders now");
    let said = app
        .status
        .as_ref()
        .map(|s| s.0.as_str())
        .unwrap_or_default();
    assert!(said.starts_with("usage: /todo"), "{said}");
}

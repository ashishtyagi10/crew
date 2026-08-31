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

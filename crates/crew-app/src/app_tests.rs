use super::{slash_command, CrewApp};

#[test]
fn slash_command_parses() {
    assert_eq!(slash_command("/settings"), Some("settings"));
    assert_eq!(slash_command("/ settings "), Some("settings"));
    assert_eq!(slash_command("ls -la"), None);
    assert_eq!(slash_command("/"), Some(""));
}

#[test]
fn close_pane_resets_modes_when_empty() {
    let mut app = CrewApp {
        zoomed: true,
        broadcast: true,
        ..Default::default()
    };
    app.input.broadcast = true;
    assert!(!app.close_pane(0));
    assert!(!app.zoomed && !app.broadcast && !app.input.broadcast);
    assert!(app.input.focused);
}

#[test]
fn zoom_chord_toggles() {
    let mut app = CrewApp::default();
    assert!(!app.zoomed);
    app.handle_super_chord("z");
    assert!(app.zoomed);
    app.handle_super_chord("z");
    assert!(!app.zoomed);
}

#[test]
fn cd_in_input_changes_cwd_and_legend() {
    let base = std::env::temp_dir().canonicalize().unwrap();
    let mut app = CrewApp {
        cwd: base.clone(),
        ..Default::default()
    };
    // a `cd` to an existing dir is intercepted (not forwarded) and updates state.
    assert!(!app.submit_input("cd .".to_string()));
    assert_eq!(app.cwd, base);
    assert_eq!(app.input.cwd, crate::cwd::display(&base));
    // a non-`cd` line is not treated as a directory change.
    assert!(!app.try_change_dir("ls -la"));
}

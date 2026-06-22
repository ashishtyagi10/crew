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
    assert_eq!(app.input.cwd, base);
    // a non-`cd` line is not treated as a directory change.
    assert!(!app.try_change_dir("ls -la"));
}

#[test]
fn submit_without_a_shell_hints() {
    let mut app = CrewApp::default();
    // a plain command with no terminal open is not silently dropped.
    assert!(!app.submit_input("ls".to_string()));
    assert!(app.active_status().is_some());
}

#[test]
fn cd_dash_toggles_previous_directory() {
    let base = std::env::temp_dir();
    let a = base.join("crew_cd_dash_a");
    let b = base.join("crew_cd_dash_b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();
    let (a, b) = (a.canonicalize().unwrap(), b.canonicalize().unwrap());

    let mut app = CrewApp {
        cwd: a.clone(),
        ..Default::default()
    };
    // move to b, then `cd -` returns to a, then toggles forward to b again.
    assert!(!app.submit_input(format!("cd {}", b.to_str().unwrap())));
    assert_eq!(app.cwd, b);
    assert!(!app.submit_input("cd -".to_string()));
    assert_eq!(app.cwd, a);
    assert!(!app.submit_input("cd -".to_string()));
    assert_eq!(app.cwd, b);
}

use super::quote;
use crate::app::CrewApp;
use crate::todopane::item::TodoItem;
use crate::todopane::store;

fn status(app: &CrewApp) -> String {
    app.status.clone().map(|(m, _)| m).unwrap_or_default()
}

fn item(id: u64, title: &str, project: Option<&str>) -> TodoItem {
    TodoItem {
        id,
        title: title.into(),
        project: project.map(str::to_string),
        created_ms: id,
        ..Default::default()
    }
}

#[test]
fn a_task_is_quoted_as_one_word_apostrophes_and_all() {
    assert_eq!(quote("fix it"), "'fix it'");
    assert_eq!(quote("don't"), r#"'don'\''t'"#);
}

#[test]
fn an_item_with_no_project_says_so_and_opens_nothing() {
    let _g = store::test_guard(vec![item(1, "call mum", None)]);
    let mut app = CrewApp::default();
    app.run_todo(1);
    assert!(status(&app).contains("no @project"), "{}", status(&app));
    assert!(app.panes.is_empty());
    assert!(
        store::snapshot()[0].run.is_none(),
        "nothing ran, nothing recorded"
    );
}

#[test]
fn a_project_crew_cannot_place_stops_with_the_binding_command() {
    let _g = store::test_guard(vec![item(1, "fix it", Some("nowhere-such"))]);
    let mut app = CrewApp {
        cwd: std::env::temp_dir(),
        ..Default::default()
    };
    app.run_todo(1);
    let s = status(&app);
    assert!(
        s.contains("no directory") && s.contains("/todo project nowhere-such"),
        "{s}"
    );
    assert!(app.panes.is_empty());
}

#[test]
fn todo_run_with_no_row_and_no_tag_explains_itself() {
    let _g = store::test_guard(vec![]);
    let mut app = CrewApp::default();
    app.todo_run_command(None);
    assert!(
        status(&app).starts_with("usage: /todo run"),
        "{}",
        status(&app)
    );
    app.todo_project_command("crew", "/no/such/dir/anywhere");
    assert!(status(&app).contains("not a directory"), "{}", status(&app));
}

/// Poll pane 0's pty until its output holds `marker`; panics after a
/// generous deadline so a hang names the marker.
#[cfg(unix)]
fn wait_for(app: &mut CrewApp, marker: &str) -> String {
    use crate::pane::PaneContent;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    let mut seen = String::new();
    loop {
        if let Some(PaneContent::Terminal(t)) = app.panes.get_mut(0).map(|p| &mut p.content) {
            t.pty.start_capture();
            t.pty.try_read();
            seen.push_str(&t.pty.take_capture());
        }
        if seen.contains(marker) {
            return seen;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "never saw {marker:?} in: {seen:?}"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

/// The pane a run opens sits IN the project's directory and is labelled
/// for it: a stand-in agent (`sh -c pwd`) prints the directory the pane
/// was spawned in. Drives a real PTY: Unix-only by construction.
#[cfg(unix)]
#[test]
fn a_run_opens_its_pane_in_the_project_directory() {
    let t = tempfile::tempdir().unwrap();
    let dir = t.path().canonicalize().unwrap();
    let mut app = CrewApp::default();
    let args = vec![
        "-c".to_string(),
        "echo RUN_$PWD_END".replace("$PWD", "$(pwd)"),
    ];
    assert!(app.run_cli_in("sh", &args, "@x \u{b7} sh", &dir));
    assert_eq!(app.panes[0].label.as_deref(), Some("@x \u{b7} sh"));
    let marker = format!("RUN_{}_END", dir.display());
    wait_for(&mut app, &marker);
}

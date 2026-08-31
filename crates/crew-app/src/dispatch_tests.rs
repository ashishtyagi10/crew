use crate::app::CrewApp;

/// An unrecognised command must SAY so. It used to fall through in
/// silence, which looked exactly like a command that ran and had nothing
/// to report — and would have hidden a palette row whose dispatch arm was
/// deleted.
#[test]
fn an_unknown_command_says_so_and_guesses() {
    let mut app = CrewApp::default();
    app.run_slash_command("setings");
    let s = app.status.clone().expect("a status was set").0;
    assert!(s.contains("unknown command /setings"), "{s}");
    assert!(s.contains("/settings"), "a near miss should be named: {s}");

    app.run_slash_command("wobblefish");
    let s = app.status.clone().expect("a status was set").0;
    assert!(s.contains("unknown command /wobblefish"), "{s}");
    assert!(!s.contains("did you mean"), "no guess from nonsense: {s}");
}

/// `/restart` is gone (merged into `/update`), but typing it must teach,
/// not fall through to the fuzzy matcher — which would suggest /restore,
/// a different action entirely.
#[test]
fn restart_is_a_migration_stub_pointing_at_update() {
    let mut app = CrewApp::default();
    let exit = app.run_slash_command("restart");
    assert!(!exit, "the stub must not exit or relaunch anything");
    let s = app.status.clone().expect("a status was set").0;
    assert!(s.contains("/update"), "{s}");
    assert!(
        !s.contains("unknown"),
        "not an unknown command, a merged one: {s}"
    );
}

// --- /gamma ---------------------------------------------------------------

// --- glass ----------------------------------------------------------------

/// An unreadable config value must not render the app flat with no
/// explanation — it falls back to the default strength.
#[test]
fn unknown_configured_level_falls_back() {
    let mut app = CrewApp::default();
    app.config.glass = "chunky".to_string();
    assert_eq!(app.config.glass_level(), crew_theme::GlassLevel::Medium);
}

// --- /todo done -----------------------------------------------------------

fn last_todo(app: &CrewApp) -> &crate::todopane::TodoPane {
    match &app.panes.last().expect("a pane spawned").content {
        crate::pane::PaneContent::Todo(t) => t,
        _ => panic!("expected a todo pane"),
    }
}

#[test]
fn todo_done_opens_the_history_view_with_an_optional_filter() {
    let _g = crate::todopane::store::test_guard(vec![]);
    let mut app = CrewApp::default();
    app.run_slash_command("todo");
    assert!(!last_todo(&app).done_view, "bare /todo is the active list");

    app.run_slash_command("todo done");
    let t = last_todo(&app);
    assert!(t.done_view, "/todo done opens the history");
    assert_eq!(t.filter, None);

    app.run_slash_command("todo done @crew");
    let t = last_todo(&app);
    assert!(t.done_view);
    assert_eq!(t.filter.as_deref(), Some("crew"), "the arg pre-filters");
}

/// `/todo show` / `/todo hide` are the typed half of the header button —
/// they act on the todo pane you are looking at and say what happened.
#[test]
fn todo_show_and_hide_flip_the_done_items_on_the_open_pane() {
    let _g = crate::todopane::store::test_guard(vec![]);
    let mut app = CrewApp::default();
    app.run_slash_command("todo");
    let opened = app.panes.len();

    app.run_slash_command("todo show");
    assert_eq!(app.panes.len(), opened, "reuses the open list, no new pane");
    assert!(last_todo(&app).show_done, "/todo show reveals them");

    app.run_slash_command("todo hide");
    assert!(!last_todo(&app).show_done, "/todo hide puts them back");
    let s = app.status.clone().expect("a status was set").0;
    assert!(s.contains("done items hidden"), "{s}");
}

/// From a cold start it opens the list first — the command works before
/// any todo pane exists, the way `/todo done` does.
#[test]
fn todo_show_spawns_a_list_when_none_is_open() {
    let _g = crate::todopane::store::test_guard(vec![]);
    let mut app = CrewApp::default();
    app.run_slash_command("todo show");
    let t = last_todo(&app);
    assert!(t.show_done);
    assert!(!t.done_view, "the list, not the history");
}

/// The history is done-only; asking to show done items there means "put
/// me back on the list with them in it", not "nothing to do".
#[test]
fn todo_show_walks_back_out_of_the_history_view() {
    let _g = crate::todopane::store::test_guard(vec![]);
    let mut app = CrewApp::default();
    app.run_slash_command("todo done");
    assert!(last_todo(&app).done_view);
    app.run_slash_command("todo show");
    let t = last_todo(&app);
    assert!(!t.done_view, "left the log");
    assert!(t.show_done, "with the done items on the list");
}

#[test]
fn a_bad_todo_arg_teaches_the_usage_instead_of_spawning() {
    let _g = crate::todopane::store::test_guard(vec![]);
    let mut app = CrewApp::default();
    let before = app.panes.len();
    app.run_slash_command("todo wobble");
    assert_eq!(app.panes.len(), before, "no pane from a bad arg");
    let s = app.status.clone().expect("a status was set").0;
    assert!(s.contains("usage: /todo"), "{s}");
}

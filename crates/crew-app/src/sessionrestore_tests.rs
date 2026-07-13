use std::path::PathBuf;

use crate::app::CrewApp;
use crate::pane::PaneContent;
use crate::sessionsave::SavedPane;

fn tmp_dir_str() -> String {
    std::env::temp_dir().to_string_lossy().into_owned()
}

#[test]
fn restore_from_reopens_each_kind_and_keeps_the_tracked_cwd() {
    let mut app = CrewApp {
        cwd: PathBuf::from("/"),
        ..Default::default()
    };
    app.restore_from(vec![
        SavedPane::shell(tmp_dir_str()),
        SavedPane::far(tmp_dir_str()),
    ]);
    assert_eq!(app.panes.len(), 2);
    assert!(matches!(app.panes[0].content, PaneContent::Terminal(_)));
    assert!(matches!(app.panes[1].content, PaneContent::Far(_)));
    // The shell recorded its spawn dir; the Far pane opened on it too.
    assert_eq!(
        app.panes[0].dir.as_deref(),
        Some(std::path::Path::new(tmp_dir_str().as_str()))
    );
    if let PaneContent::Far(f) = &app.panes[1].content {
        assert_eq!(f.active_cwd(), PathBuf::from(tmp_dir_str()));
    }
    // The tracked cwd is put back.
    assert_eq!(app.cwd, PathBuf::from("/"));
}

#[test]
fn restore_from_nothing_is_a_status_no_op() {
    let mut app = CrewApp::default();
    app.restore_from(Vec::new());
    assert!(app.panes.is_empty());
}

#[test]
fn unknown_kind_spawns_nothing() {
    // load_at filters unknown kinds, but restore_from is belt-and-braces
    // for direct callers.
    let mut app = CrewApp::default();
    app.restore_from(vec![SavedPane {
        kind: "hologram".into(),
        dir: None,
        min: false,
    }]);
    assert!(app.panes.is_empty());
}

#[test]
fn session_panes_snapshots_a_crew_chat_pane_by_its_routing_label() {
    // Review-found Critical: detection matched label=="crew" but
    // spawn_crew_pane only set `name` — crew panes silently never saved.
    // spawn_plugin_pane now stamps the routing label; this pins the pair.
    use crate::chat::ChatPane;
    use crate::pane::Pane;
    use crew_plugin::Plugin;
    let plugin = Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()]).unwrap();
    let mut app = CrewApp::default();
    app.panes.push(Pane {
        content: PaneContent::Chat(ChatPane::new(plugin, "crew".into())),
        grid: crew_term::GridSize { cols: 80, rows: 24 },
        rect: crate::layout::Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        },
        label: Some("crew".to_string()),
        name: Some("crew".to_string()),
        dir: None,
        activity: false,
        bell: false,
        hidden: false,
        attention: None,
    });
    assert_eq!(app.session_panes(), vec![SavedPane::crew()]);
    // A label-less chat pane (Cmd+J) is NOT snapshot.
    app.panes[0].label = None;
    assert!(app.session_panes().is_empty());
}

#[test]
fn minimized_panes_restore_minimized_and_focus_lands_visible() {
    let mut app = CrewApp {
        cwd: PathBuf::from("/"),
        ..Default::default()
    };
    let mut min_shell = SavedPane::shell(tmp_dir_str());
    min_shell.min = true;
    // Visible first, minimized LAST — the loop leaves the minimized one
    // focused, which reconcile_grid's focus-restores rule would un-minimize;
    // restore must land focus back on a visible pane.
    app.restore_from(vec![SavedPane::shell(tmp_dir_str()), min_shell]);
    assert_eq!(app.panes.len(), 2);
    assert!(!app.panes[0].hidden);
    assert!(app.panes[1].hidden);
    assert_eq!(app.focused, 0, "focus must land on the visible pane");
}

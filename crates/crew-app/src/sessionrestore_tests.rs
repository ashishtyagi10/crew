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
    }]);
    assert!(app.panes.is_empty());
}

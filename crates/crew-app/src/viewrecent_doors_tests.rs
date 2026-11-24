//! Every door into a viewer notes its file: `/view` (Cmd+P's pick, a click)
//! and the Explorer's reused preview, which swaps the file inside
//! a viewer already on screen without going through `/view`. A synthetic
//! file (`/about`) is taken back off.
use std::path::Path;

#[test]
fn two_doors_both_note_the_file() {
    let _g = crate::app::theme_test_guard();
    let dir = std::env::temp_dir().join(format!("crew_viewrecent_doors_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let (one, two) = (dir.join("one.md"), dir.join("two.md"));
    std::fs::write(&one, b"# one").unwrap();
    std::fs::write(&two, b"# two").unwrap();
    let mut app = crate::app::CrewApp::default();
    app.open_view(&one.to_string_lossy());
    let at = |p: &Path| {
        let p = std::fs::canonicalize(p).unwrap();
        super::list().iter().position(|q| q == &p)
    };
    assert!(at(&one).is_some(), "/view noted it");
    // The Explorer's preview: the viewer just opened is reused for `two`.
    app.sidebar.explorer.preview = app.panes.last().map(|p| p.born_ms);
    let panes = app.panes.len();
    app.explorer_open(two.clone());
    assert_eq!(
        app.panes.len(),
        panes,
        "the preview was reused, not a new pane"
    );
    assert!(
        at(&two).unwrap() < at(&one).unwrap(),
        "the Explorer noted it, newest"
    );
    app.spawn_about_pane();
    let crate::pane::PaneContent::View(v) = &app.panes.last().unwrap().content else {
        panic!("a viewer");
    };
    assert!(
        at(&v.path).is_none(),
        "the changelog is not a file anybody chose"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// A session restore puts viewers back without noting them: the older file
/// stays older (noting it would put it in front, and write the list).
#[test]
fn a_restore_does_not_reorder_the_list() {
    let _g = crate::app::theme_test_guard();
    let dir = std::env::temp_dir().join(format!("crew_viewrecent_rest_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let (old, new) = (dir.join("old.md"), dir.join("new.md"));
    std::fs::write(&old, b"# old").unwrap();
    std::fs::write(&new, b"# new").unwrap();
    super::note(&old);
    super::note(&new);
    let mut app = crate::app::CrewApp::default();
    app.restore_from(vec![crate::sessionsave::SavedPane {
        kind: "view".into(),
        dir: Some(old.to_string_lossy().into_owned()),
        ..Default::default()
    }]);
    assert_eq!(app.panes.len(), 1, "the viewer is back");
    // Other tests open viewers meanwhile, so the pair's order is the proof.
    let at = |p: &Path| {
        let p = std::fs::canonicalize(p).unwrap();
        super::list().iter().position(|q| q == &p).unwrap()
    };
    assert!(at(&new) < at(&old), "{:?}", super::list());
    let _ = std::fs::remove_dir_all(&dir);
}

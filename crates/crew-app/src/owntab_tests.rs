//! `/far` and `/inbox` open in a tab of their own.
use crate::app::CrewApp;
use crate::pane::{Pane, PaneContent};

fn tile() -> Pane {
    Pane {
        glide: crate::glide::Glide::default(),
        content: PaneContent::Todo(crate::todopane::TodoPane::new()),
        grid: crew_term::GridSize { cols: 80, rows: 24 },
        rect: crate::spawn::PLACEHOLDER_RECT,
        label: None,
        name: None,
        dir: None,
        activity: false,
        bell: false,
        hidden: false,
        attention: None,
        born_ms: 0,
        zoom: None,
        tab: 0,
    }
}

/// One pane, on screen, in the first tab.
fn one_pane() -> (CrewApp, u32) {
    let mut app = CrewApp::default();
    app.panes = vec![tile()];
    app.focused = 0;
    app.input.focused = false;
    app.reconcile_grid();
    let first = app.tabs.active;
    (app, first)
}

#[test]
fn the_inbox_opens_in_a_tab_of_its_own_and_is_found_there_again() {
    let (mut app, first) = one_pane();
    app.inbox_command("");
    app.reconcile_grid();
    assert_eq!(app.tabs.list.len(), 2);
    assert_ne!(app.tabs.active, first, "the inbox's tab is on screen");
    assert_eq!(
        app.panes[0].tab, first,
        "the pane that was there stayed put"
    );
    assert_eq!(app.panes[1].tab, app.tabs.active);
    assert!(
        app.in_active_tab(1) && !app.in_active_tab(0),
        "the inbox has its tab to itself"
    );

    // Asked for again from the first tab, the same pane comes forward: one
    // inbox, one tab for it.
    app.activate_tab(first);
    app.inbox_command("");
    app.reconcile_grid();
    assert_eq!((app.panes.len(), app.tabs.list.len()), (2, 2));
    assert_eq!(app.focused, 1);
    assert_ne!(app.tabs.active, first);
}

#[test]
fn far_opens_in_a_tab_of_its_own() {
    let (mut app, first) = one_pane();
    app.cwd = std::env::temp_dir();
    app.far_in_own_tab();
    app.reconcile_grid();
    assert_eq!(app.panes.len(), 2, "the Far pane opened");
    assert!(matches!(app.panes[1].content, PaneContent::Far(_)));
    assert_eq!(app.tabs.list.len(), 2);
    assert_eq!(app.panes[0].tab, first);
    assert_eq!(app.panes[1].tab, app.tabs.active);
    assert_ne!(app.tabs.active, first);
}

/// With nothing open the tab on screen is already empty; a second one would
/// leave a blank tab behind the pane.
#[test]
fn on_an_empty_window_they_open_in_the_tab_already_there() {
    let mut app = CrewApp::default();
    app.inbox_command("");
    app.reconcile_grid();
    assert_eq!(app.tabs.list.len(), 1);
}

/// A file opened from a pane is read in a tab of its own, and closing it
/// comes back to the pane it was opened from.
#[test]
fn a_file_opens_in_its_own_tab_and_closing_it_comes_back() {
    let (mut app, first) = one_pane();
    let path = std::env::temp_dir().join(format!("crew-owntab-{}.md", std::process::id()));
    std::fs::write(&path, "# read me\n").unwrap();
    app.open_view(&path.to_string_lossy());
    app.reconcile_grid();
    assert!(matches!(app.panes[1].content, PaneContent::View(_)));
    assert_eq!(app.tabs.list.len(), 2);
    assert_eq!(app.panes[0].tab, first, "the pane it came from stayed put");
    assert!(app.in_active_tab(1) && !app.in_active_tab(0));

    app.close_pane(1);
    app.reconcile_grid();
    assert_eq!((app.tabs.list.len(), app.tabs.active), (1, first));
    assert_eq!(app.focused, 0);
}

/// A path that is not a file opens nothing — and leaves no empty tab behind.
#[test]
fn a_missing_file_opens_no_tab() {
    let (mut app, first) = one_pane();
    app.open_view("/no/such/crew/file.md");
    app.reconcile_grid();
    assert_eq!((app.panes.len(), app.tabs.list.len()), (1, 1));
    assert_eq!(app.tabs.active, first);
}

/// A restore puts a viewer back in the tab it is restoring into.
#[test]
fn a_restored_viewer_stays_in_the_tab_on_screen() {
    let (mut app, first) = one_pane();
    let path = std::env::temp_dir().join(format!("crew-owntab-r-{}.md", std::process::id()));
    std::fs::write(&path, "# restored\n").unwrap();
    app.open_view_here(&path.to_string_lossy());
    app.reconcile_grid();
    assert_eq!(app.tabs.list.len(), 1);
    assert_eq!(app.panes[1].tab, first);
}

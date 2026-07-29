use crate::app::CrewApp;
use crate::pane::PaneContent;

#[test]
#[allow(clippy::field_reassign_with_default)] // test fixture: set cwd after default()
fn open_view_pushes_a_zoomed_focused_pane() {
    let dir = std::env::temp_dir();
    let f = dir.join("open-view-test.txt");
    std::fs::write(&f, "hi\n").unwrap();
    let mut app = CrewApp::default();
    app.cwd = dir;
    app.open_view("open-view-test.txt");
    assert_eq!(app.panes.len(), 1);
    assert!(matches!(app.panes[0].content, PaneContent::View(_)));
    assert!(app.zoomed, "the viewer opens zoomed");
}

#[test]
#[allow(clippy::field_reassign_with_default)] // test fixture: set cwd after default()
fn a_missing_file_opens_no_pane_and_says_why() {
    let mut app = CrewApp::default();
    app.cwd = std::env::temp_dir();
    app.open_view("definitely-not-here.txt");
    assert!(app.panes.is_empty(), "no empty pane for a missing file");
    let status = app.active_status().unwrap_or_default().to_string();
    assert!(
        status.contains("definitely-not-here.txt"),
        "the status names the file: {status}",
    );
}

#[test]
fn an_empty_argument_is_a_usage_hint() {
    let mut app = CrewApp::default();
    app.open_view("");
    assert!(app.panes.is_empty());
    let status = app.active_status().unwrap_or_default().to_string();
    assert!(status.contains("/view"), "got {status}");
}

#[test]
#[allow(clippy::field_reassign_with_default)] // test fixture: set cwd after default()
fn a_loading_pane_keeps_the_animation_gate_open_and_a_settled_one_does_not() {
    // wants_animation_frame IS the "an idle crew never repaints" invariant.
    // A skeleton that is not registered will not animate; one that never
    // deregisters burns the GPU forever.
    let dir = std::env::temp_dir();
    let f = dir.join("anim-gate-test.txt");
    std::fs::write(&f, "hi\n").unwrap();
    let mut app = CrewApp::default();
    app.cwd = dir;
    app.open_view("anim-gate-test.txt");
    assert!(
        app.wants_animation_frame(crate::anim::now_ms()),
        "a loading pane animates"
    );
    for _ in 0..500 {
        if let PaneContent::View(v) = &mut app.panes[0].content {
            if v.poll() {
                break;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    // Every freshly spawned pane also gets a short "just appeared" card
    // assemble animation (`paneview::spawn_timeline`, 380ms at the default
    // Full motion level) — a timer completely unrelated to `ViewPane`'s own
    // loading state, but one that ALSO holds `wants_animation_frame` open.
    // A tiny fixture file settles well inside that 380ms window, so without
    // waiting it out this assertion would fail even for a fully correct
    // `ViewPane::animating()` — not what this test means to check. Wait past
    // it so the assertion below isolates the invariant this test is named
    // for: the VIEWER's own gate, not the pane-spawn chrome every pane kind
    // shares.
    std::thread::sleep(std::time::Duration::from_millis(420));
    assert!(
        !app.wants_animation_frame(crate::anim::now_ms()),
        "a settled pane stops asking for frames"
    );
}

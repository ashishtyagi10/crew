use crate::app::CrewApp;
use crate::pane::PaneContent;

#[test]
fn open_view_pushes_a_zoomed_focused_pane() {
    let dir = std::env::temp_dir();
    let f = dir.join("open-view-test.txt");
    std::fs::write(&f, "hi\n").unwrap();
    let mut app = CrewApp {
        cwd: dir,
        ..Default::default()
    };
    app.open_view("open-view-test.txt");
    assert_eq!(app.panes.len(), 1);
    assert!(matches!(app.panes[0].content, PaneContent::View(_)));
    assert_eq!(app.focused, 0, "the new pane is focused");
    assert!(app.zoomed, "the viewer opens zoomed");
}

#[test]
fn a_missing_file_opens_no_pane_and_says_why() {
    let mut app = CrewApp {
        cwd: std::env::temp_dir(),
        ..Default::default()
    };
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
fn a_viewer_never_holds_the_animation_gate_open_once_its_spawn_animation_settles() {
    // `wants_animation_frame` IS the "an idle crew never repaints" invariant.
    // A `ViewPane` has no animation of its own to register: its `Loading`
    // state renders one static "loading…" banner with no time input
    // (`viewpane::lines::for_state`), `Pane::cells` takes no `now`, and
    // `lines_for` caches on `(cols, raw)` only — so there is nothing for an
    // animation arm to actually redraw differently frame to frame. The only
    // animation a fresh viewer pane gets is the ordinary "just appeared"
    // card-assemble timeline every pane kind shares
    // (`paneview::spawn_timeline`, 380ms at the default Full motion level).
    // Once THAT settles, the gate must close — whether the file is still
    // loading or has already landed. This is what would catch someone
    // re-adding a `PaneContent::View(v) => v.animating()` arm later.
    let dir = std::env::temp_dir();
    let f = dir.join("anim-gate-test.txt");
    std::fs::write(&f, "hi\n").unwrap();
    let mut app = CrewApp {
        cwd: dir,
        ..Default::default()
    };
    app.open_view("anim-gate-test.txt");

    // Past the pane-spawn assemble window (380ms at Full motion) — the only
    // animation source a new pane has.
    std::thread::sleep(std::time::Duration::from_millis(420));

    // Force the pane back into Loading (`reload` doesn't touch `born_ms`,
    // so the spawn window stays elapsed) to prove the gate stays shut even
    // mid-load, not merely once the file has settled.
    if let PaneContent::View(v) = &mut app.panes[0].content {
        v.reload();
        assert!(v.loading(), "reload re-arms the loader");
    }
    assert!(
        !app.wants_animation_frame(crate::anim::now_ms()),
        "a still-loading viewer must not hold the animation gate open"
    );

    // And once that reload lands, same thing.
    for _ in 0..500 {
        if let PaneContent::View(v) = &mut app.panes[0].content {
            if v.poll() {
                break;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(
        !app.wants_animation_frame(crate::anim::now_ms()),
        "a settled viewer does not hold the gate open either"
    );
}

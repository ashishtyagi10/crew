//! `reload_views_after_edit`: a viewer whose `$EDITOR` pane has ended
//! re-reads the file, and a viewer whose editor is still running is left
//! alone. Included from `poll.rs` via `#[path]` (see there) rather than
//! `poll.rs`'s own sibling `poll_tests.rs`, because everything here drives
//! `ViewPane`/`PaneContent::View` state directly and belongs next to the
//! rest of the viewer's test suite.
use crate::app::CrewApp;
use crate::pane::PaneContent;
// `Pane` is only named by `editor_pane`, which is Unix-only (it spawns `sh`).
#[cfg(unix)]
use crate::pane::Pane;

/// Spin `poll()` on every `View` pane until a tick reports no change, so a
/// test can assert on settled state (`Ready`, not `Loading`) without a fixed
/// sleep. Bounded rather than unbounded — a stuck loader must not hang the
/// test suite.
///
/// "No change this tick" is not by itself proof of settling: right after
/// `reload()` starts a fresh worker, the very first `poll()` legitimately
/// finds the channel still empty (the thread hasn't run yet) and reports
/// `false` — indistinguishable, from that bool alone, from an already-Ready
/// pane with nothing new. So this also requires every pane to have left
/// `Loading`; otherwise it keeps spinning instead of returning on a false
/// "settled".
fn settle(app: &mut CrewApp) {
    for _ in 0..500 {
        let mut changed = false;
        let mut loading = false;
        for p in &mut app.panes {
            if let PaneContent::View(v) = &mut p.content {
                changed |= v.poll();
                loading |= v.loading();
            }
        }
        if !changed && !loading {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

#[test]
fn a_viewer_reloads_when_its_editor_pane_goes_away() {
    let dir = std::env::temp_dir();
    let f = dir.join("reload-after-edit.txt");
    std::fs::write(&f, "before\n").unwrap();
    let mut app = CrewApp {
        cwd: dir,
        ..Default::default()
    };
    app.open_view("reload-after-edit.txt");
    settle(&mut app);

    // Pretend an editor pane was spawned for it and has since exited: a
    // born_ms no live terminal pane carries.
    if let PaneContent::View(v) = &mut app.panes[0].content {
        v.editor_born = Some(1);
    }
    std::fs::write(&f, "after\n").unwrap();
    assert!(app.reload_views_after_edit(), "the exit triggers a reload");
    settle(&mut app);
    match &app.panes[0].content {
        PaneContent::View(v) => match &v.state {
            crate::viewpane::LoadState::Ready { loaded, .. } => {
                assert_eq!(loaded.text, "after\n", "the viewer shows the edited file")
            }
            _ => panic!("settled"),
        },
        _ => panic!("still a viewer"),
    }
}

// Uses `editor_pane`, which spawns a real `sh` PTY: Unix-only by
// construction — Windows has no `sh` for that spawn to succeed on.
#[cfg(unix)]
#[test]
fn a_viewer_is_left_alone_while_its_editor_pane_is_still_running() {
    // The mirror case: an editor pane with the SAME born_ms is still alive
    // (its terminal still reports a foreground command), so no reload should
    // fire and the viewer must keep showing the pre-edit content.
    let dir = std::env::temp_dir();
    let f = dir.join("reload-while-editing.txt");
    std::fs::write(&f, "before\n").unwrap();
    let mut app = CrewApp {
        cwd: dir,
        ..Default::default()
    };
    app.open_view("reload-while-editing.txt");
    settle(&mut app);

    if let PaneContent::View(v) = &mut app.panes[0].content {
        v.editor_born = Some(7);
    }
    app.panes.push(editor_pane(7, Some("vim")));
    std::fs::write(&f, "after\n").unwrap();

    assert!(
        !app.reload_views_after_edit(),
        "a still-running editor must not trigger a reload"
    );
    settle(&mut app);
    match &app.panes[0].content {
        PaneContent::View(v) => match &v.state {
            crate::viewpane::LoadState::Ready { loaded, .. } => assert_eq!(
                loaded.text, "before\n",
                "the viewer keeps showing the pre-edit content while the editor runs"
            ),
            _ => panic!("settled"),
        },
        _ => panic!("still a viewer"),
    }
}

// Uses `editor_pane`, which spawns a real `sh` PTY: Unix-only by
// construction — Windows has no `sh` for that spawn to succeed on.
#[cfg(unix)]
#[test]
fn a_freshly_spawned_editor_survives_the_pre_scan_window() {
    // Real `TermPane.cmd` is `None` from the moment a terminal pane is
    // spawned (`spawn.rs`) until `procname`'s throttled scan (~1x/s, gated
    // by `ProcNames::due`) fills it in. `reload_views_after_edit` runs on
    // EVERY tick — roughly every 16ms — so the very first tick after `e`
    // spawns the editor sees `cmd: None`. A liveness check keyed only on
    // `cmd.is_some()` would read that as "the editor already exited" and
    // reload immediately, before the edit even started, clearing
    // `editor_born` so the real exit later goes unnoticed. This reproduces
    // that exact sequence: a pane born "now" (i.e. inside the grace window)
    // with `cmd` still `None`.
    let dir = std::env::temp_dir();
    let f = dir.join("reload-pre-scan.txt");
    std::fs::write(&f, "before\n").unwrap();
    let mut app = CrewApp {
        cwd: dir,
        ..Default::default()
    };
    app.open_view("reload-pre-scan.txt");
    settle(&mut app);

    let born = crate::anim::now_ms();
    if let PaneContent::View(v) = &mut app.panes[0].content {
        v.editor_born = Some(born);
    }
    app.panes.push(editor_pane(born, None));

    assert!(
        !app.reload_views_after_edit(),
        "a just-spawned editor (cmd still None) must not look exited before procname's first scan"
    );
    match &app.panes[0].content {
        PaneContent::View(v) => assert_eq!(
            v.editor_born,
            Some(born),
            "the editor↔viewer association must survive the pre-scan window"
        ),
        _ => panic!("still a viewer"),
    }
}

#[cfg(unix)]
/// A minimal terminal pane standing in for an `$EDITOR` pane: `cmd` mirrors
/// what `procname` would have filled in (or not yet) at `born_ms`.
fn editor_pane(born_ms: u64, cmd: Option<&str>) -> Pane {
    use crate::app::FALLBACK_SIZE;
    use crate::pane::TermPane;
    use crate::spawn::PLACEHOLDER_RECT;
    use crew_term::PtyTerm;

    let pty = PtyTerm::spawn(FALLBACK_SIZE, "sh").expect("spawn a shell for the fake editor pane");
    let input = pty.writer();
    Pane {
        glide: crate::glide::Glide::default(),
        content: PaneContent::Terminal(Box::new(TermPane {
            pty,
            input,
            cmd: cmd.map(str::to_string),
            cmd_since: None,
            tail: Default::default(),
            read_at: 0,
            spans: Default::default(),
            trail: Default::default(),
        })),
        grid: FALLBACK_SIZE,
        rect: PLACEHOLDER_RECT,
        label: None,
        name: None,
        dir: None,
        activity: false,
        bell: false,
        hidden: false,
        attention: None,
        born_ms,
    }
}

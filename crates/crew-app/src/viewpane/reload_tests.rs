//! `reload_views_after_edit`: a viewer whose `$EDITOR` pane has ended
//! re-reads the file, and a viewer whose editor is still running is left
//! alone. Included from `poll.rs` via `#[path]` (see there) rather than
//! `poll.rs`'s own sibling `poll_tests.rs`, because everything here drives
//! `ViewPane`/`PaneContent::View` state directly and belongs next to the
//! rest of the viewer's test suite.
use crate::app::CrewApp;
use crate::pane::{Pane, PaneContent};

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
    app.panes.push(editor_pane(7));
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

/// A minimal terminal pane standing in for a live `$EDITOR`: `cmd: Some(_)`
/// is what `reload_views_after_edit` treats as "still running", born at
/// `born_ms` to match the viewer's `editor_born`.
fn editor_pane(born_ms: u64) -> Pane {
    use crate::app::FALLBACK_SIZE;
    use crate::pane::TermPane;
    use crate::spawn::PLACEHOLDER_RECT;
    use crew_term::PtyTerm;

    let pty = PtyTerm::spawn(FALLBACK_SIZE, "sh").expect("spawn a shell for the fake editor pane");
    let input = pty.writer();
    Pane {
        content: PaneContent::Terminal(Box::new(TermPane {
            pty,
            input,
            cmd: Some("vim".to_string()),
            cmd_since: None,
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

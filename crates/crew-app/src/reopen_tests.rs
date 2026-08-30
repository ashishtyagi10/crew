use std::path::PathBuf;

use crate::app::CrewApp;
use crate::farpane::FarPane;
use crate::layout::Rect;
use crate::pane::{Pane, PaneContent};
use crate::reopen::{saved_for, ClosedStack, DEPTH};
use crate::sessionsave::SavedPane;
use crew_term::GridSize;

fn tmp() -> PathBuf {
    std::env::temp_dir()
}

/// A pane with the given content and no PTY behind it — everything the undo
/// stack reads is on `Pane` itself.
fn pane(content: PaneContent) -> Pane {
    Pane {
        glide: crate::glide::Glide::default(),
        content,
        grid: GridSize { cols: 40, rows: 10 },
        rect: Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        },
        label: None,
        name: None,
        dir: None,
        activity: false,
        bell: false,
        hidden: false,
        attention: None,
        born_ms: crate::anim::now_ms(),
    }
}

fn settings() -> crate::settingspane::SettingsPane {
    crate::settingspane::SettingsPane::new(Default::default(), Vec::new())
}

fn far() -> Pane {
    pane(PaneContent::Far(FarPane::new(tmp())))
}

#[test]
fn a_far_pane_is_remembered_by_the_directory_it_was_showing() {
    let sp = saved_for(&far()).expect("a Far pane is restorable");
    assert_eq!(sp.kind, "far");
    assert_eq!(sp.dir.as_deref(), Some(tmp().to_string_lossy().as_ref()));
    assert!(!sp.remote);
}

/// A pane kind with no honest restore is not made restorable by carrying a
/// directory — the kind decides, not the field.
#[test]
fn a_settings_pane_is_not_restorable_however_much_directory_it_carries() {
    let mut p = pane(PaneContent::Settings(settings()));
    p.dir = Some(tmp());
    assert_eq!(saved_for(&p), None);
}

/// The one thing that separates undo-close from session restore: a shell is
/// remembered by the pane's own tracked `dir` — kept current by the OSC 7 the
/// shell emits on every `cd` — because the process whose cwd `session_panes`
/// would ask the OS for is being reaped as we look. A shell that never
/// reported one is not remembered at all, rather than reopened somewhere it
/// never was.
///
/// Drives a real PTY running a POSIX shell: Unix-only by construction.
#[cfg(unix)]
#[test]
fn a_shell_is_remembered_by_its_tracked_directory() {
    use crate::pane::TermPane;
    use crew_term::PtyTerm;

    let pty = PtyTerm::spawn_in(
        GridSize { cols: 40, rows: 10 },
        "/bin/sh",
        &[],
        Some(&tmp()),
    )
    .expect("spawn a shell");
    let input = pty.writer();
    let mut p = pane(PaneContent::Terminal(Box::new(TermPane {
        pty,
        input,
        cmd: None,
        cmd_since: None,
        tail: Default::default(),
        read_at: 0,
        spans: Default::default(),
        trail: Default::default(),
    })));
    assert_eq!(saved_for(&p), None, "no tracked directory, nothing to say");
    p.dir = Some(tmp());
    let sp = saved_for(&p).expect("a shell with a directory is restorable");
    assert_eq!(sp.kind, "shell");
    assert_eq!(sp.dir.as_deref(), Some(tmp().to_string_lossy().as_ref()));
}

#[test]
fn a_pane_with_no_honest_restore_is_never_remembered() {
    let mut stack = ClosedStack::default();
    stack.remember(&pane(PaneContent::Settings(settings())));
    assert_eq!(stack.len(), 0);
    assert!(stack.take().is_none());
    // …and the next real close is still the one `/reopen` reaches.
    stack.remember(&far());
    assert_eq!(stack.len(), 1);
    assert_eq!(stack.take().map(|c| c.saved.kind), Some("far".to_string()));
}

#[test]
fn the_stack_keeps_the_newest_and_drops_the_oldest() {
    let mut stack = ClosedStack::default();
    for i in 0..DEPTH + 3 {
        let mut p = far();
        p.name = Some(format!("pane {i}"));
        stack.remember(&p);
    }
    assert_eq!(stack.len(), DEPTH, "the stack is bounded");
    // Walking it back reaches exactly DEPTH panes, newest first, and the
    // three oldest are gone rather than merely unreachable.
    let titles: Vec<String> = std::iter::from_fn(|| stack.take())
        .map(|c| c.title)
        .collect();
    assert_eq!(titles.first().map(String::as_str), Some("pane 10"));
    assert_eq!(titles.last().map(String::as_str), Some("pane 3"));
    assert_eq!(titles.len(), DEPTH);
}

/// A pane closed while minimized into the nav comes back onto the grid:
/// reopening is an act of wanting to see it.
#[test]
fn a_minimized_pane_reopens_visible() {
    let mut p = far();
    p.hidden = true;
    let sp = saved_for(&p).expect("restorable");
    assert!(!sp.min);
}

#[test]
fn reopen_with_nothing_closed_says_so() {
    let mut app = CrewApp::default();
    app.reopen_pane();
    assert!(app.panes.is_empty());
    assert_eq!(
        app.status.as_ref().map(|(s, _)| s.as_str()),
        Some("nothing to reopen")
    );
}

/// End to end through the real spawn path: close a restored Far pane and
/// `/reopen` brings a Far pane back on the same directory, with the tracked
/// cwd left exactly where it was.
#[test]
fn closing_a_pane_and_reopening_it_puts_it_back() {
    let mut app = CrewApp {
        cwd: PathBuf::from("/"),
        ..Default::default()
    };
    app.restore_from(vec![SavedPane::far(tmp().to_string_lossy().into_owned())]);
    assert_eq!(app.panes.len(), 1);
    app.close_pane(0);
    assert!(app.panes.is_empty());
    assert_eq!(app.closed.len(), 1, "the close was written down");

    app.reopen_pane();
    assert_eq!(app.panes.len(), 1, "the pane came back");
    assert!(matches!(app.panes[0].content, PaneContent::Far(_)));
    assert_eq!(app.closed.len(), 0, "and the stack was spent");
    assert_eq!(app.cwd, PathBuf::from("/"), "the tracked cwd is put back");
    assert!(!app.zoomed);
    let status = app.status.as_ref().map(|(s, _)| s.clone()).unwrap();
    assert!(status.starts_with("reopened "), "status was {status:?}");
    assert!(
        !status.contains("more"),
        "nothing else is waiting: {status:?}"
    );
}

/// `/only` does not go through `close_pane`, so it records its own
/// casualties — and `/reopen` walks them back one at a time.
#[test]
fn only_can_be_undone_pane_by_pane() {
    let mut app = CrewApp {
        cwd: PathBuf::from("/"),
        ..Default::default()
    };
    let dir = tmp().to_string_lossy().into_owned();
    app.restore_from(vec![
        SavedPane::far(dir.clone()),
        SavedPane::far(dir.clone()),
        SavedPane::far(dir),
    ]);
    assert_eq!(app.panes.len(), 3);
    app.focused = 0;
    app.close_other_panes(); // arms the confirmation
    app.close_other_panes(); // …and answers it
    assert_eq!(app.panes.len(), 1);
    assert_eq!(app.closed.len(), 2, "both casualties were written down");

    app.reopen_pane();
    assert_eq!(app.panes.len(), 2);
    assert!(
        app.status
            .as_ref()
            .is_some_and(|(s, _)| s.contains("(1 more)")),
        "the status counts what is still undoable: {:?}",
        app.status
    );
    app.reopen_pane();
    assert_eq!(app.panes.len(), 3, "the grid is back");
    assert_eq!(app.closed.len(), 0);
}

/// Cmd+Shift+T is the chord, and it arrives as its own shifted character.
#[test]
fn the_chord_reaches_reopen() {
    let mut app = CrewApp::default();
    assert!(!app.handle_super_chord("T"), "Cmd+Shift+T never exits");
    assert_eq!(
        app.status.as_ref().map(|(s, _)| s.as_str()),
        Some("nothing to reopen"),
        "the chord ran the command"
    );
}

//! Fix-with-smith on a headless terminal: real OSC 133 marks fed to a real
//! parser, then the pane asked what failed. Nothing here spawns a broker.
use super::*;
use crate::app::CrewApp;
use crate::pane::{Pane, TermPane};
use crew_term::{GridSize, PtyTerm, ShellMark, TermModel};

pub(super) const CELL: (f32, f32) = (8.0, 17.0);

pub(super) fn pane() -> Pane {
    let grid = GridSize { cols: 40, rows: 8 };
    let args = vec!["-c".to_string(), "exec cat > /dev/null".to_string()];
    let pty = PtyTerm::spawn_in(grid, "/bin/sh", &args, Some(&std::env::temp_dir())).unwrap();
    let input = pty.writer();
    Pane {
        glide: crate::glide::Glide::default(),
        content: PaneContent::Terminal(Box::new(TermPane {
            pty,
            input,
            cmd: None,
            cmd_since: None,
            tail: Default::default(),
            read_at: 0,
            spans: Default::default(),
            trail: Default::default(),
            find: None,
            images: Default::default(),
        })),
        grid,
        rect: crate::layout::Rect {
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
        born_ms: 0,
        zoom: None,
        tab: 0,
    }
}

/// A terminal pane in an app with a frame, its screen not yet scrolled.
pub(super) fn bare_app() -> CrewApp {
    let mut app = CrewApp {
        geo_override: Some((CELL.0, CELL.1, 1280.0, 720.0, 1.0)),
        ..Default::default()
    };
    app.panes = vec![pane()];
    app.input.focused = false;
    app.reconcile_grid();
    app.build_frame();
    app
}

/// Pane 0 ran `line`, printed `out`, and the shell said it exited `code` —
/// the same steps `poll` takes on the real marks. `fill` lines first: a
/// shell's screen is full, so each output line scrolls one into history and
/// the span's line numbers (`lines_seen`) move with it.
pub(super) fn ran(app: &mut CrewApp, fill: usize, (line, out, code): (&str, &str, i32)) {
    let PaneContent::Terminal(t) = &mut app.panes[0].content else {
        unreachable!()
    };
    let old: String = (0..fill).map(|i| format!("old {i}\r\n")).collect();
    t.pty.feed(format!("{old}$ {line}\r\n").as_bytes());
    t.spans.started("cargo".into(), t.pty.lines_seen(), 0);
    t.spans.set_line(Some(line.into()));
    let end = format!("{out}\x1b]133;D;{code}\x07$ ");
    t.pty.feed(end.as_bytes());
    for mark in t.pty.take_shell() {
        if let ShellMark::Done(c) = mark {
            t.spans.finished(c, t.pty.lines_seen(), 1);
        }
    }
    app.build_frame();
}

/// [`bare_app`] after one command on a full screen.
pub(super) fn app_after(line: &str, out: &str, code: i32) -> CrewApp {
    let mut app = bare_app();
    ran(&mut app, 80, (line, out, code));
    app
}

pub(super) const OUT: &str = "running 1 test\r\nerror[E0433]: nope\r\n";

#[test]
fn a_failed_command_in_a_real_terminal_is_found_with_its_own_output() {
    let mut app = app_after("cargo test -p crew-app", OUT, 101);
    let f = app.failure_of(0).expect("exit 101 is a failure");
    assert_eq!(
        (f.command.as_str(), f.exit),
        ("cargo test -p crew-app", 101)
    );
    assert_eq!(f.output, "running 1 test\nerror[E0433]: nope", "{f:?}");
}

#[test]
fn a_command_that_succeeded_is_not_a_failure() {
    let mut app = app_after("cargo test", OUT, 0);
    assert_eq!(app.failure_of(0), None);
}

#[test]
fn explain_sends_the_failure_not_the_screen_when_the_command_failed() {
    let mut app = app_after("cargo test -p crew-app", OUT, 101);
    let ctx = app.explain_context(0).unwrap();
    assert!(
        ctx.starts_with("`cargo test -p crew-app` failed (exit 101)"),
        "{ctx}"
    );
    assert!(ctx.contains("error[E0433]"), "{ctx}");
}

#[test]
fn explain_keeps_the_screen_tail_when_nothing_failed() {
    let mut app = app_after("cargo test", OUT, 0);
    let ctx = app.explain_context(0).unwrap();
    assert!(
        ctx.contains("error[E0433]") && !ctx.contains("failed (exit"),
        "{ctx}"
    );
    assert!(app.explain_context(9).is_err(), "no such pane");
}

#[test]
fn a_failure_on_a_screen_that_has_not_scrolled_still_carries_its_output() {
    // A fresh pane, or one just cleared: no line has scrolled, so the span
    // counts no lines. The failure must not arrive empty.
    let mut app = bare_app();
    ran(&mut app, 0, ("cargo test", OUT, 101));
    let f = app.failure_of(0).expect("a failure");
    assert!(f.output.contains("error[E0433]: nope"), "{f:?}");
}

#[test]
fn fixing_a_pane_that_did_not_fail_opens_nothing() {
    let mut app = app_after("cargo test", OUT, 0);
    app.fix_with_smith(0, None);
    assert_eq!(app.panes.len(), 1, "no smith pane was spawned");
}

#[test]
fn the_prefill_lands_in_smiths_composer_and_is_not_sent() {
    let plugin = crew_plugin::Plugin::spawn("cat", &[]).unwrap();
    let mut app = CrewApp::default();
    let mut smith = pane();
    smith.content = PaneContent::Chat(crate::chat::ChatPane::new(plugin, "crew".into()));
    smith.label = Some("crew".into());
    app.panes = vec![smith];
    assert!(app.prefill_smith("`cargo test` failed (exit 101). Fix it."));
    let PaneContent::Chat(c) = &mut app.panes[0].content else {
        unreachable!()
    };
    assert_eq!(c.input, "`cargo test` failed (exit 101). Fix it.");
    assert!(c.messages.is_empty(), "nothing was sent");
    c.input = "my note".into();
    assert!(app.prefill_smith("failure"));
    let PaneContent::Chat(c) = &app.panes[0].content else {
        unreachable!()
    };
    assert_eq!(c.input, "my note\nfailure", "under the draft, not over it");
}

#[test]
fn with_no_smith_pane_there_is_nowhere_to_prefill() {
    let mut app = CrewApp::default();
    assert!(!app.prefill_smith("x"));
}

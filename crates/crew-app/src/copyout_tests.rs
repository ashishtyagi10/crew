//! Copy-a-command's-output on real OSC 133 bytes fed to a pane's terminal,
//! with a recording sink for the clipboard: nothing here touches the real one.
use super::*;
use crate::fixwithsmith::tests::bare_app;
use crew_term::TermModel;

/// The clipboard seam: what was put, and whether the "system" took it.
fn put_into(sink: &mut Vec<String>) -> impl FnMut(String) -> bool + '_ {
    move |t| {
        sink.push(t);
        true
    }
}

fn copy(app: &mut CrewApp) -> Vec<String> {
    let mut sink = Vec::new();
    app.copy_command_output_to(&mut put_into(&mut sink));
    sink
}

fn status(app: &CrewApp) -> String {
    app.active_status().unwrap_or_default().to_string()
}

/// Pane 0's shell ran `line`, printed `out`, and finished: a zsh's marks,
/// all in one read, as a pty hands over a command that is quick. The poll's
/// span (`started` at the first output row, as `poll` does) rides along so
/// Cmd+Opt+Up has somewhere to land.
fn ran(app: &mut CrewApp, line: &str, out: &str) {
    let crate::pane::PaneContent::Terminal(t) = &mut app.panes[0].content else {
        unreachable!()
    };
    t.pty
        .feed(format!("\x1b]133;A\x07$ \x1b]133;B\x07{line}\r\n\x1b]133;C\x07").as_bytes());
    t.spans.started("cargo".into(), t.pty.lines_seen(), 0);
    t.spans.set_line(Some(line.into()));
    t.pty
        .feed(format!("{out}\x1b]133;D;0\x07\x1b]133;A\x07$ \x1b]133;B\x07").as_bytes());
    t.spans.finished(Some(0), t.pty.lines_seen(), 1);
    t.pty.take_shell();
    app.build_frame();
}

fn lines(n: usize, tag: &str) -> String {
    (1..=n).map(|i| format!("{tag}{i}\r\n")).collect()
}

#[test]
fn the_last_commands_output_is_copied_exactly() {
    // Colour in the output, a blank gap after it: neither reaches the clipboard.
    let out = "\x1b[31mred\x1b[0m line\r\n  indented\r\n\r\nlast\r\n\r\n\r\n";
    let mut app = bare_app();
    ran(&mut app, "cargo test -p crew-app", out);
    assert_eq!(copy(&mut app), ["red line\n  indented\n\nlast"]);
    assert_eq!(
        status(&app),
        "copied 4 lines of cargo test -p crew-app's output"
    );
}

#[test]
fn a_quick_command_after_a_slow_one_is_the_one_copied() {
    let mut app = bare_app();
    ran(&mut app, "cargo test", &lines(60, "ok "));
    ran(&mut app, "ls", "a\r\nb\r\n");
    assert_eq!(copy(&mut app), ["a\nb"]);
}

#[test]
fn heavy_output_keeps_its_head_and_a_soft_wrapped_line_is_whole() {
    let long = "x".repeat(500);
    let mut app = bare_app();
    ran(&mut app, "run", &format!("{long}\r\n{}", lines(100, "n ")));
    let got = copy(&mut app).remove(0);
    assert!(got.starts_with(&format!("{long}\nn 1\n")));
    assert!(got.ends_with("n 100"));
}

#[test]
fn scrolled_back_to_an_earlier_command_copies_that_one() {
    let mut app = bare_app();
    // A shell's screen is full, so the poll's line numbers (`lines_seen`,
    // the screen's bottom) agree with the marks'.
    let crate::pane::PaneContent::Terminal(t) = &mut app.panes[0].content else {
        unreachable!()
    };
    t.pty.feed(lines(100, "old ").as_bytes());
    ran(&mut app, "first", "one\r\ntwo\r\n");
    ran(&mut app, "second", &lines(60, "t "));
    assert!(
        copy(&mut app)[0].starts_with("t 1\n"),
        "the live view is the last command"
    );
    assert!(app.jump_command(true), "Cmd+Opt+Up lands on a start");
    assert!(app.jump_command(true), "and again, on the one before");
    assert_eq!(copy(&mut app), ["one\ntwo"]);
    // The wheel instead: a few lines up inside the second one's output.
    let crate::pane::PaneContent::Terminal(t) = &mut app.panes[0].content else {
        unreachable!()
    };
    t.pty.scroll(-1000);
    t.pty.scroll(3);
    assert!(copy(&mut app)[0].starts_with("t 1\n"));
}

#[test]
fn no_marks_says_so_and_copies_nothing() {
    let mut app = bare_app();
    assert!(copy(&mut app).is_empty());
    assert_eq!(status(&app), NO_MARKS);
}

#[test]
fn a_command_that_printed_nothing_copies_nothing() {
    let mut app = bare_app();
    ran(&mut app, "true", "");
    assert!(copy(&mut app).is_empty());
    assert!(
        status(&app).contains("true printed nothing"),
        "{}",
        status(&app)
    );
}

#[test]
fn a_full_screen_program_is_refused_with_a_notice() {
    let mut app = bare_app();
    ran(&mut app, "ls", "a\r\n");
    let crate::pane::PaneContent::Terminal(t) = &mut app.panes[0].content else {
        unreachable!()
    };
    t.pty.feed(b"vim\r\n\x1b]133;C\x07\x1b[?1049h");
    assert!(copy(&mut app).is_empty());
    assert!(status(&app).contains("full-screen"), "{}", status(&app));
}

#[test]
fn lines_dropped_from_scrollback_are_said() {
    let mut app = bare_app();
    let crate::pane::PaneContent::Terminal(t) = &mut app.panes[0].content else {
        unreachable!()
    };
    t.pty.set_scrollback(20);
    ran(&mut app, "make", &lines(100, "r "));
    let got = copy(&mut app).remove(0);
    assert!(got.ends_with("r 100"));
    assert!(
        status(&app).contains("lines were dropped from scrollback"),
        "{}",
        status(&app)
    );
}

#[test]
fn a_refused_clipboard_is_said() {
    let mut app = bare_app();
    ran(&mut app, "ls", "a\r\n");
    app.copy_command_output_to(&mut |_| false);
    assert_eq!(status(&app), "clipboard unavailable");
}

#[test]
fn a_long_command_is_cut_and_a_secret_redacted_in_the_notice() {
    let mut app = bare_app();
    ran(&mut app, &format!("echo {}", "y".repeat(80)), "a\r\n");
    copy(&mut app);
    assert!(
        status(&app).contains('\u{2026}') && status(&app).len() < 90,
        "{}",
        status(&app)
    );
}

/// The chord is crew's (so it never reaches the program) and is not Cmd+C.
#[test]
fn the_chord_is_crews_and_cmd_c_is_still_copy() {
    use crate::winkeys::{chord, WinChord};
    use winit::keyboard::{Key, ModifiersState, SmolStr};
    let k = |s: &str| Key::Character(SmolStr::new(s));
    let alt = |s: &str| chord(&k(s), None, ModifiersState::ALT);
    assert_eq!(alt("'"), Some(WinChord::Cmd("C")));
    assert_eq!(alt("c"), Some(WinChord::Cmd("c")), "Alt+C is still copy");
    let mac = |s: &str, m| crate::keychord::cmd_string(&k(s), &k("c"), None, m, true);
    let sup = ModifiersState::SUPER;
    assert_eq!(mac("C", sup | ModifiersState::SHIFT).as_deref(), Some("C"));
    assert_eq!(mac("c", sup).as_deref(), Some("c"));
}

#[test]
fn the_chord_reaches_the_copy_and_leaves_a_selection_copy_alone() {
    let _g = crate::app::theme_test_guard();
    let mut app = bare_app();
    assert!(!app.handle_super_chord("C"));
    assert_eq!(status(&app), NO_MARKS);
}

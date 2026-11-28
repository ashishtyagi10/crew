//! Copy-a-command's-output on real OSC 133 marks fed to a headless terminal,
//! with a recording sink for the clipboard: nothing here touches the real one.
use super::*;
use crate::fixwithsmith::tests::{app_after, bare_app, ran};

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

#[test]
fn the_last_commands_output_is_copied_exactly() {
    // Colour in the output, a blank gap after it: neither reaches the clipboard.
    let out = "\x1b[31mred\x1b[0m line\r\n  indented\r\n\r\nlast\r\n\r\n\r\n";
    let mut app = app_after("cargo test -p crew-app", out, 0);
    assert_eq!(copy(&mut app), ["red line\n  indented\n\nlast"]);
    assert_eq!(
        status(&app),
        "copied 4 lines of cargo test -p crew-app's output"
    );
}

#[test]
fn a_soft_wrapped_line_is_copied_whole() {
    // Far wider than any pane: several rows, one line.
    let long = "x".repeat(500);
    let mut app = app_after("run", &format!("{long}\r\nshort\r\n"), 0);
    assert_eq!(copy(&mut app), [format!("{long}\nshort")]);
}

#[test]
fn a_failed_command_copies_too() {
    let mut app = app_after("cargo test", "error[E0433]: nope\r\n", 101);
    assert_eq!(copy(&mut app), ["error[E0433]: nope"]);
}

#[test]
fn scrolled_back_to_an_earlier_command_copies_that_one() {
    let mut app = app_after("first", "one\r\ntwo\r\n", 0);
    ran(&mut app, 60, ("second", "three\r\n", 0));
    assert_eq!(
        copy(&mut app),
        ["three"],
        "the live view is the last command"
    );
    assert!(
        app.jump_command(true),
        "Cmd+Opt+Up lands on the earlier start"
    );
    assert_eq!(copy(&mut app), ["one\ntwo"]);
}

#[test]
fn no_marks_says_so_and_copies_nothing() {
    let mut app = bare_app();
    assert!(copy(&mut app).is_empty());
    assert_eq!(status(&app), NO_MARKS);
}

#[test]
fn a_command_the_shell_never_finished_has_no_honest_end() {
    // Seen by the process poll alone (no `D`): not cut out on a guess.
    let mut app = bare_app();
    ran(&mut app, 0, ("sleep", "x\r\n", 0));
    let crate::pane::PaneContent::Terminal(t) = &mut app.panes[0].content else {
        unreachable!()
    };
    t.spans.0[0].exit = None;
    assert!(copy(&mut app).is_empty());
    assert_eq!(status(&app), NO_MARKS);
}

#[test]
fn a_command_that_printed_nothing_copies_nothing() {
    let mut app = app_after("true", "", 0);
    assert!(copy(&mut app).is_empty());
    assert!(status(&app).contains("printed nothing"), "{}", status(&app));
}

#[test]
fn a_refused_clipboard_is_said() {
    let mut app = app_after("ls", "a\r\n", 0);
    app.copy_command_output_to(&mut |_| false);
    assert_eq!(status(&app), "clipboard unavailable");
}

#[test]
fn a_long_command_is_cut_in_the_notice() {
    let line = format!("echo {}", "y".repeat(80));
    let mut app = app_after(&line, "a\r\n", 0);
    copy(&mut app);
    assert!(
        status(&app).contains('\u{2026}') && status(&app).len() < 90,
        "{}",
        status(&app)
    );
}

#[test]
fn pick_takes_the_newest_finished_command_at_or_above_the_view() {
    let mut spans = Spans::default();
    for (from, exit) in [(10, Some(0)), (20, Some(1)), (30, None)] {
        spans.started("c".into(), from, 0);
        spans.0.last_mut().unwrap().exit = exit;
    }
    assert_eq!(pick(&spans, None), Some(1), "unfinished 30 is skipped");
    assert_eq!(pick(&spans, Some(25)), Some(1));
    assert_eq!(pick(&spans, Some(12)), Some(0));
    assert_eq!(pick(&spans, Some(5)), None, "above every command");
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

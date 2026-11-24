//! The gates past the setting: who holds the keys, whether the window is
//! KNOWN to be focused, how often one pane is answered, and whether its last
//! answer has been taken yet. Each counts clipboard reads, so "dropped" also
//! proves "not looked at".
#![cfg_attr(not(unix), allow(dead_code, unused_imports))]
use super::pane_tests::{running, until_titled, Sink};
use super::tests::fake;
use super::{READ_CAP, READ_GAP};
use crate::app::CrewApp;
use crate::pane::PaneContent;
use std::time::{Duration, Instant};

/// Ask once, then title the pane `asked`.
const ASK: &str = "printf '\\033]52;c;?\\007\\033]2;asked\\007'; sleep 5";

/// The script that asks `n` times, titling itself `1`, `2`… after each.
fn asks(n: usize, raw: bool) -> String {
    let mut s = String::from(if raw { "stty raw -echo; " } else { "" });
    for k in 1..=n {
        s += &format!("printf '\\033]52;c;?\\007\\033]2;{k}\\007'; sleep 0.4; ");
    }
    s + "sleep 5"
}

fn allowed() -> CrewApp {
    let mut app = CrewApp::default();
    app.config.clipboard_read = "allow".into();
    app.win_focus = Some(true);
    app
}

/// No pane visible, the bar has the keys, `focused` still names a pane
/// minimized into the nav: that pane is not answered.
#[cfg(unix)]
#[test]
fn the_input_bar_holding_the_keys_answers_no_pane() {
    let sink = Sink::default();
    let mut app = allowed();
    app.panes.push(running(ASK, Some(&sink)));
    app.panes[0].hidden = true;
    app.input.focused = true;
    until_titled(&mut app, "asked");
    let mut reads = 0;
    app.serve_clipboard_asks(&mut fake("hi", &mut reads), Instant::now());
    assert_eq!(
        reads, 0,
        "a pane read the clipboard while the bar had the keys"
    );
    assert!(sink.0.lock().unwrap().is_empty());
}

/// Before the platform has said the window is focused, nothing is read:
/// a background launch fails closed.
#[cfg(unix)]
#[test]
fn an_unknown_window_focus_answers_nothing() {
    let sink = Sink::default();
    let mut app = allowed();
    app.win_focus = None;
    app.panes.push(running(ASK, Some(&sink)));
    until_titled(&mut app, "asked");
    let mut reads = 0;
    app.serve_clipboard_asks(&mut fake("hi", &mut reads), Instant::now());
    assert_eq!(reads, 0, "read before the window was known to be focused");
}

/// Three asks on a fake clock: the one inside the gap is dropped, unread;
/// the one after it is answered.
#[cfg(unix)]
#[test]
fn one_pane_is_answered_at_most_once_per_gap() {
    let sink = Sink::default();
    let mut app = allowed();
    app.panes.push(running(&asks(3, false), Some(&sink)));
    let t0 = Instant::now();
    let mut reads = 0;
    for (k, at) in [(1, 0), (2, 100), (3, 300)] {
        until_titled(&mut app, &k.to_string());
        let now = t0 + Duration::from_millis(at);
        app.serve_clipboard_asks(&mut fake("hi", &mut reads), now);
    }
    assert!(READ_GAP > Duration::from_millis(100) && READ_GAP <= Duration::from_millis(300));
    assert_eq!(reads, 2, "the ask inside the gap was read");
    assert_eq!(
        *sink.0.lock().unwrap(),
        b"\x1b]52;c;aGk=\x07\x1b]52;c;aGk=\x07"
    );
}

/// A program that asks and never reads: its 1.4 MB answer sits in the
/// queue, and a second ask — however late — is dropped, not stacked behind.
#[cfg(unix)]
#[test]
fn no_second_answer_while_the_first_is_still_unread() {
    let mut app = allowed();
    app.panes.push(running(&asks(2, true), None));
    let big = "a".repeat(READ_CAP);
    let t0 = Instant::now();
    let mut reads = 0;
    until_titled(&mut app, "1");
    app.serve_clipboard_asks(&mut fake(&big, &mut reads), t0);
    until_titled(&mut app, "2");
    let PaneContent::Terminal(t) = &app.panes[0].content else {
        unreachable!()
    };
    assert!(t.pty.input_backlog() > 0, "the first answer was taken");
    app.serve_clipboard_asks(&mut fake(&big, &mut reads), t0 + Duration::from_secs(5));
    assert_eq!(reads, 1, "a second answer was stacked behind the first");
}

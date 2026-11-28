//! The Mac's composed text reaches the focused shell or the input bar.
#![cfg_attr(not(unix), allow(dead_code, unused_imports))]
use super::ime_wanted;
use crate::app::CrewApp;
use crate::pane::{Pane, PaneContent, TermPane};
use crew_term::{GridSize, PtyTerm};
use std::io::Write;
use std::sync::{Arc, Mutex};
use winit::event::Ime;

/// The bytes a terminal pane was sent, in place of its pty's writer.
#[derive(Clone, Default)]
struct Sink(Arc<Mutex<Vec<u8>>>);

impl Write for Sink {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn shell(sink: &Sink) -> Pane {
    let grid = GridSize { cols: 40, rows: 8 };
    let pty = PtyTerm::spawn_in(grid, "/bin/cat", &[], Some(&std::env::temp_dir())).unwrap();
    let content = PaneContent::Terminal(Box::new(TermPane {
        pty,
        input: Box::new(sink.clone()),
        cmd: None,
        cmd_since: None,
        tail: Default::default(),
        read_at: 0,
        spans: Default::default(),
        trail: Default::default(),
        find: None,
        images: Default::default(),
    }));
    Pane {
        glide: crate::glide::Glide::default(),
        content,
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

fn commit(s: &str) -> Ime {
    Ime::Commit(s.to_string())
}

/// Option+e then e on a Mac: the é the input system commits is the shell's.
#[cfg(unix)]
#[test]
fn a_commit_types_into_the_focused_shell() {
    let sink = Sink::default();
    let mut a = CrewApp {
        panes: vec![shell(&sink)],
        focused: 0,
        ..Default::default()
    };
    a.input.focused = false;
    a.on_ime(Ime::Preedit("´".into(), Some((0, 2))));
    assert!(sink.0.lock().unwrap().is_empty(), "the accent waits");
    a.on_ime(commit("é"));
    assert_eq!(*sink.0.lock().unwrap(), [0xc3, 0xa9]);
}

/// The input bar takes the letter when it holds focus; the shell does not.
#[cfg(unix)]
#[test]
fn a_commit_goes_to_the_input_bar_when_it_has_focus() {
    let sink = Sink::default();
    let mut a = CrewApp {
        panes: vec![shell(&sink)],
        ..Default::default()
    };
    a.input.focused = true;
    a.input.text = "caf".into();
    a.on_ime(commit("é"));
    assert_eq!(a.input.text, "café");
    assert!(sink.0.lock().unwrap().is_empty());
}

/// A Mac takes composed text unless Option is Meta, which must stay a key
/// press for `altmeta`; elsewhere winit composes into the key press itself.
#[test]
fn ime_is_on_for_a_mac_unless_option_is_meta() {
    assert!(ime_wanted(true, false));
    assert!(!ime_wanted(true, true));
    assert!(!ime_wanted(false, false));
}

/// IME follows focus: only the shell and the input bar take a commit, and
/// not while help reads keys as commands — a dead key there would be held
/// as preedit and lost.
#[cfg(unix)]
#[test]
fn only_a_shell_or_the_input_bar_takes_text() {
    let sink = Sink::default();
    let mut a = CrewApp {
        panes: vec![shell(&sink)],
        ..Default::default()
    };
    a.input.focused = false;
    assert!(a.takes_text());
    a.help_open = true;
    assert!(!a.takes_text());
    a.on_ime(commit("é"));
    assert!(sink.0.lock().unwrap().is_empty(), "help holds the keys");
    a.help_open = false;
    a.panes.clear();
    assert!(!a.takes_text(), "no shell, no bar");
}

/// A composition begun in one shell moves with focus to another: the window's IME stays on
/// and the OS keeps the marked text, so its commit lands where focus went.
#[cfg(unix)]
#[test]
fn moving_focus_carries_the_composition() {
    let _g = crate::app::theme_test_guard();
    let sink = Sink::default();
    let mut a = CrewApp {
        panes: vec![shell(&sink), shell(&sink)],
        focused: 0,
        ..Default::default()
    };
    a.input.focused = false;
    a.on_ime(Ime::Preedit("か".into(), None));
    a.sync_ime();
    assert!(a.ime.preedit.is_some(), "still its owner's");
    a.focused = 1;
    a.sync_ime();
    let owner = a.ime.preedit.as_ref().map(|p| p.owner);
    assert_eq!(owner, Some(crate::imepreedit::Owner::Pane(1)));
}

/// A program that asked for every kitty flag (a Neovim's `1|2|8|16`) still
/// gets a composed letter: the IME commit is text, never an escape code.
#[cfg(unix)]
#[test]
fn a_commit_still_types_under_every_kitty_flag() {
    let sink = Sink::default();
    let mut pane = shell(&sink);
    let grid = GridSize { cols: 40, rows: 8 };
    let script = "printf '\\033[>27u'; exec cat";
    let pty = PtyTerm::spawn_in(
        grid,
        "/bin/sh",
        &["-c".to_string(), script.to_string()],
        None,
    )
    .unwrap();
    if let PaneContent::Terminal(t) = &mut pane.content {
        t.pty = pty;
    }
    let mut a = CrewApp {
        panes: vec![pane],
        focused: 0,
        ..Default::default()
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let flags = |a: &mut CrewApp| match &mut a.panes[0].content {
        PaneContent::Terminal(t) => {
            t.pty.try_read();
            t.pty.input_modes().kitty_flags
        }
        _ => 0,
    };
    while flags(&mut a) != 27 && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert_eq!(flags(&mut a), 27, "the program's push reached the pane");
    a.on_ime(commit("é"));
    assert_eq!(*sink.0.lock().unwrap(), [0xc3, 0xa9]);
}

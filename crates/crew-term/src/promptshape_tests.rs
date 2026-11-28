#![cfg(unix)]
use super::*;
use crate::GridSize;
use std::io::Write;
use std::time::{Duration, Instant};

fn pump_until(term: &mut PtyTerm, done: impl Fn(&PtyTerm) -> bool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        term.try_read();
        if done(term) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}

/// lazygit killed with a hand over a button: the shell has the pane back, so
/// the hand goes (and with it nothing else was left to report).
#[test]
fn a_dead_programs_pointer_shape_goes_when_the_shell_is_back() {
    let mut term = PtyTerm::spawn(GridSize { cols: 60, rows: 8 }, "sh").unwrap();
    let mut w = term.writer();
    w.write_all(b"printf '\\033]22;pointer\\007'; echo LEFT\n")
        .unwrap();
    w.flush().unwrap();
    assert!(pump_until(&mut term, |t| t.pointer_shape().is_some()));
    assert_eq!(term.pointer_shape(), Some("pointer"));
    assert!(term.release_leftover_modes());
    assert_eq!(term.pointer_shape(), None);
    assert!(!term.release_leftover_modes(), "nothing left");
}

/// On the alternate screen the program may still own it.
#[test]
fn the_alternate_screen_keeps_its_pointer_shape() {
    let mut term = PtyTerm::spawn(GridSize { cols: 60, rows: 8 }, "sh").unwrap();
    let mut w = term.writer();
    w.write_all(b"printf '\\033[?1049h\\033]22;grab\\007'\n")
        .unwrap();
    w.flush().unwrap();
    assert!(pump_until(&mut term, |t| t.pointer_shape().is_some()));
    assert!(!term.release_leftover_modes());
    assert_eq!(term.pointer_shape(), Some("grab"));
}

/// A TUI that leaves the alternate screen takes its hand with it, with no
/// shell mark involved.
#[test]
fn leaving_the_alternate_screen_clears_the_program_shape() {
    let mut term = PtyTerm::spawn(GridSize { cols: 60, rows: 8 }, "sh").unwrap();
    term.core.feed(b"\x1b[?1049h\x1b]22;pointer\x07");
    assert_eq!(term.pointer_shape(), Some("pointer"));
    term.core.feed(b"\x1b[?1049l");
    assert_eq!(term.pointer_shape(), None);
}

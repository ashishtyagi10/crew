use super::*;
use crate::GridSize;
use std::io::Write;
use std::time::{Duration, Instant};

fn modes(alt_screen: bool, mouse: bool, focus_events: bool) -> InputModes {
    InputModes {
        alt_screen,
        mouse,
        focus_events,
        ..InputModes::default()
    }
}

#[test]
fn only_left_reporting_on_the_primary_screen_is_cleared() {
    assert_eq!(leftover_reset(modes(false, false, false), false), None);
    assert_eq!(
        leftover_reset(modes(false, true, false), false),
        Some(RESET)
    );
    assert_eq!(
        leftover_reset(modes(false, false, true), false),
        Some(RESET)
    );
    // Scheme notifications alone are a leftover too.
    assert_eq!(
        leftover_reset(modes(false, false, false), true),
        Some(RESET)
    );
    // A full-screen program may still be the one that asked.
    assert_eq!(leftover_reset(modes(true, true, true), true), None);
    // Keys are the shell's own business: DECCKM alone is left on.
    let keys = InputModes {
        app_cursor: true,
        ..InputModes::default()
    };
    assert_eq!(leftover_reset(keys, false), None);
}

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

/// A real shell on a real pty: a program turns on SGR mouse, drag, motion
/// and focus reporting and exits without turning them off. They stay on
/// (the shell never resets them) until crew sees the shell back, and then
/// every one is off, while the shell's own app-cursor mode is kept.
#[test]
fn a_dead_programs_mouse_and_focus_modes_go_when_the_shell_is_back() {
    let mut term = PtyTerm::spawn(GridSize { cols: 60, rows: 8 }, "sh").unwrap();
    let mut w = term.writer();
    w.write_all(b"printf '\\033[?1h\\033[?1003h\\033[?1006h\\033[?1004h'; echo LEFT\n")
        .unwrap();
    w.flush().unwrap();
    assert!(pump_until(&mut term, |t| {
        let m = t.input_modes();
        m.mouse && m.sgr_mouse && m.focus_events && m.app_cursor
    }));
    // Nothing else clears them: the program is gone and they are still on.
    std::thread::sleep(Duration::from_millis(200));
    term.try_read();
    assert!(term.input_modes().mouse, "left on by the dead program");
    assert_eq!(term.foreground_pid(), None, "the shell has the pane");
    assert!(term.release_leftover_modes());
    let m = term.input_modes();
    assert!(!m.mouse && !m.mouse_drag && !m.mouse_motion, "{m:?}");
    assert!(!m.sgr_mouse && !m.focus_events, "{m:?}");
    assert!(m.app_cursor, "the shell's keys are kept: {m:?}");
    assert!(
        !term.release_leftover_modes(),
        "nothing left the second time"
    );
}

/// On the alternate screen nothing is touched: the program there may live.
#[test]
fn the_alternate_screen_keeps_its_mouse() {
    let mut term = PtyTerm::spawn(GridSize { cols: 60, rows: 8 }, "sh").unwrap();
    let mut w = term.writer();
    w.write_all(b"printf '\\033[?1049h\\033[?1000h'\n").unwrap();
    w.flush().unwrap();
    assert!(pump_until(&mut term, |t| {
        let m = t.input_modes();
        m.alt_screen && m.mouse
    }));
    assert!(!term.release_leftover_modes());
    assert!(term.input_modes().mouse);
}

/// Claude Code killed with `kill -9` leaves DECSET 2031 on, and every
/// light/dark flip after that typed `ESC [ ? 997 ; 1 n` at the prompt. The
/// shell back clears it like the mouse.
#[test]
fn a_dead_programs_scheme_notifications_go_when_the_shell_is_back() {
    let mut term = PtyTerm::spawn(GridSize { cols: 60, rows: 8 }, "sh").unwrap();
    let mut w = term.writer();
    w.write_all(b"printf '\\033[?2031h'; echo LEFT\n").unwrap();
    w.flush().unwrap();
    assert!(pump_until(&mut term, |t| t.scheme_notify_enabled()));
    assert!(!term.input_modes().mouse, "only 2031 is left on");
    assert!(term.release_leftover_modes());
    assert!(!term.scheme_notify_enabled());
    assert!(!term.release_leftover_modes(), "nothing left");
}

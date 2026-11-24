//! The cursor a dead program hid comes back with the shell (see the
//! `promptmodes` module doc): `ESC [ ? 25 l` left on after a `kill -9`.
use super::*;
use crate::model::TermCore;
use crate::GridSize;
use std::io::Write;
use std::time::{Duration, Instant};

const SIZE: GridSize = GridSize { cols: 40, rows: 6 };

/// The reset the shell-back path feeds for a core, as `release_leftover_modes`.
fn release(core: &mut TermCore) -> bool {
    let hidden = core.cursor_hidden();
    let scheme = core.scheme_notify_enabled();
    let Some(reset) = leftover_reset(core.input_modes(), scheme, hidden, false) else {
        return false;
    };
    core.feed(reset);
    true
}

#[test]
fn a_hidden_cursor_alone_is_a_leftover_on_the_primary_screen_only() {
    let plain = InputModes::default();
    assert_eq!(leftover_reset(plain, false, true, false), Some(RESET));
    // A full-screen program (vim, htop) hides it while it lives.
    let alt = InputModes {
        alt_screen: true,
        ..plain
    };
    assert_eq!(leftover_reset(alt, false, true, false), None);
}

/// No pty: the cursor the program hid is drawn again once the reset lands,
/// and a second reset finds nothing to do.
#[test]
fn the_reset_shows_the_cursor_a_dead_program_hid() {
    let mut core = TermCore::new(SIZE);
    core.feed(b"\x1b[?25lFRAME\r\n$ ");
    assert_eq!(core.cursor_cell(), None, "the program hid it");
    assert!(release(&mut core));
    assert!(!core.cursor_hidden());
    assert_eq!(core.cursor_cell(), Some((2, 1)), "drawn at the prompt");
    assert!(!release(&mut core), "nothing left the second time");
}

/// htop / less / vim killed on the alternate screen: the prompt mark takes
/// the pane back to the primary screen, which alone leaves the cursor
/// hidden; the reset the mark runs shows it.
#[test]
fn a_prompt_mark_brings_back_a_full_screen_programs_cursor() {
    let mut core = TermCore::new(SIZE);
    core.feed(b"\x1b[?1049h\x1b[?25lHTOP");
    assert!(!release(&mut core), "the program may still be running");
    assert!(core.cursor_hidden());
    core.feed(b"\x1b]133;A\x07$ ");
    assert!(!core.input_modes().alt_screen, "the mark left it");
    assert!(
        core.cursor_hidden(),
        "leaving the alternate screen keeps it hidden"
    );
    assert!(release(&mut core));
    assert!(core.cursor_cell().is_some(), "the prompt has its cursor");
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

/// A real shell: while the program that hid the cursor still runs, nothing
/// says the shell is back (a foreground job, no prompt mark) and it stays
/// hidden; once it is gone, the shell back shows it.
#[cfg(unix)]
#[test]
fn a_running_program_keeps_its_hidden_cursor_until_the_shell_is_back() {
    let mut term = PtyTerm::spawn(SIZE, "sh").unwrap();
    let mut w = term.writer();
    w.write_all(b"printf '\\033[?25l'; sleep 1\n").unwrap();
    w.flush().unwrap();
    assert!(pump_until(&mut term, |t| t.cursor_cell().is_none()));
    assert!(pump_until(&mut term, |t| t.foreground_pid().is_some()));
    assert!(term.take_shell().is_empty(), "no prompt yet");
    assert_eq!(term.cursor_cell(), None, "still the program's");
    assert!(pump_until(&mut term, |t| t.foreground_pid().is_none()));
    term.try_read();
    assert_eq!(term.cursor_cell(), None, "no shell shows it again");
    assert!(term.release_leftover_modes());
    assert!(term.cursor_cell().is_some(), "the prompt has its cursor");
}

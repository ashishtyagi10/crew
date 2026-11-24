//! The kitty keyboard flags a killed program pushed go when the shell has
//! the pane back (see the `promptmodes` module doc), from this reset only
//! when the shell sends no prompt marks.
#![cfg_attr(not(unix), allow(dead_code, unused_imports))]
use super::*;
use crate::model::TermCore;
use crate::GridSize;
use std::io::Write;
use std::time::{Duration, Instant};

const SIZE: GridSize = GridSize { cols: 40, rows: 6 };

fn kitty() -> InputModes {
    InputModes {
        kitty_keys: true,
        ..InputModes::default()
    }
}

#[test]
fn kitty_flags_alone_are_a_leftover_for_a_shell_without_marks() {
    assert_eq!(
        leftover_reset(kitty(), false, false, false),
        Some(RESET_KITTY)
    );
    // The prompt mark already emptied the stack; a push after it is fish's.
    assert_eq!(leftover_reset(kitty(), false, false, true), None);
    let alt = InputModes {
        alt_screen: true,
        ..kitty()
    };
    assert_eq!(leftover_reset(alt, false, false, false), None);
    assert!(RESET_KITTY.starts_with(RESET));
}

/// No pty: two pushes (nvim's and the plugin it ran) and one reset empties
/// the whole stack, which a single `ESC [ < u` would not.
#[test]
fn the_reset_pops_every_push() {
    let mut core = TermCore::new(SIZE);
    core.feed(b"\x1b[>1u\x1b[>1u");
    let m = core.input_modes();
    let reset = leftover_reset(m, false, false, core.command_marks()).expect("a leftover");
    core.feed(reset);
    assert!(!core.input_modes().kitty_keys);
    core.feed(b"\x1b[?u");
    assert_eq!(core.take_replies().as_deref(), Some("\x1b[?0u"));
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

/// A real shell: a program pushes kitty mode and exits without popping it.
/// It stays (the shell never pops it) until the shell back clears it.
#[cfg(unix)]
#[test]
fn a_dead_programs_kitty_flags_go_when_the_shell_is_back() {
    let mut term = PtyTerm::spawn(SIZE, "sh").unwrap();
    let mut w = term.writer();
    w.write_all(b"printf '\\033[>1u'; echo LEFT\n").unwrap();
    w.flush().unwrap();
    assert!(pump_until(&mut term, |t| t.input_modes().kitty_keys));
    assert!(pump_until(&mut term, |t| t.foreground_pid().is_none()));
    assert!(term.release_leftover_modes());
    assert!(!term.input_modes().kitty_keys);
    assert!(!term.release_leftover_modes(), "nothing left");
}

/// fish 4 before its first command: a prompt mark, then the shell's own
/// `CSI = 5 u` (no `C` / `D` mark yet, so no command marks counted). The
/// prompt mark's reset drops a left-on mouse but keeps the shell's flags.
#[cfg(unix)]
#[test]
fn the_prompt_marks_reset_keeps_the_shells_own_kitty_flags() {
    let mut term = PtyTerm::spawn(SIZE, "sh").unwrap();
    let mut w = term.writer();
    w.write_all(b"printf '\\033[?1000h'; sleep 0.2; printf '\\033]133;A\\007\\033[=5u'\n")
        .unwrap();
    w.flush().unwrap();
    assert!(pump_until(&mut term, |t| t.input_modes().kitty_keys));
    let marks = term.take_shell();
    assert!(marks.contains(&crate::osc::ShellMark::Prompt), "{marks:?}");
    assert!(term.release_prompt_leftovers());
    let m = term.input_modes();
    assert!(m.kitty_keys && !m.mouse, "{m:?}");
}

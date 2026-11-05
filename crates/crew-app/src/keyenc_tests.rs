use winit::keyboard::NamedKey::*;

use super::{named, Mods};

const NONE: Mods = Mods {
    shift: false,
    alt: false,
    ctrl: false,
};
const ALT: Mods = Mods { alt: true, ..NONE };
const SHIFT: Mods = Mods {
    shift: true,
    ..NONE
};
const CTRL: Mods = Mods { ctrl: true, ..NONE };

fn sent(n: winit::keyboard::NamedKey, m: Mods, app: bool) -> Option<String> {
    named(n, m, app, false).map(|b| String::from_utf8(b).unwrap())
}

#[test]
fn function_keys_send_xterms_sequences_with_their_modifiers() {
    assert_eq!(sent(F1, NONE, false).as_deref(), Some("\x1bOP"));
    assert_eq!(sent(F4, NONE, false).as_deref(), Some("\x1bOS"));
    assert_eq!(sent(F5, NONE, false).as_deref(), Some("\x1b[15~"));
    assert_eq!(sent(F6, NONE, false).as_deref(), Some("\x1b[17~"));
    assert_eq!(sent(F10, NONE, false).as_deref(), Some("\x1b[21~"));
    assert_eq!(sent(F11, NONE, false).as_deref(), Some("\x1b[23~"));
    assert_eq!(sent(F12, NONE, false).as_deref(), Some("\x1b[24~"));
    assert_eq!(sent(F2, SHIFT, false).as_deref(), Some("\x1b[1;2Q"));
    assert_eq!(sent(F5, CTRL, false).as_deref(), Some("\x1b[15;5~"));
}

#[test]
fn option_moves_and_deletes_a_word() {
    assert_eq!(sent(ArrowLeft, ALT, false).as_deref(), Some("\x1bb"));
    assert_eq!(sent(ArrowRight, ALT, true).as_deref(), Some("\x1bf"));
    assert_eq!(sent(Backspace, ALT, false).as_deref(), Some("\x1b\x7f"));
}

/// Application mode answers the plain cursor keys only; Shift and Ctrl
/// arrows are left to the plain table (bare shells eat their xterm forms).
#[test]
fn cursor_keys_follow_application_mode_and_leave_the_rest_alone() {
    assert_eq!(sent(ArrowUp, NONE, true).as_deref(), Some("\x1bOA"));
    assert_eq!(sent(Home, NONE, true).as_deref(), Some("\x1bOH"));
    assert_eq!(sent(ArrowUp, NONE, false), None);
    assert_eq!(sent(ArrowLeft, SHIFT, true), None);
    assert_eq!(sent(ArrowLeft, CTRL, false), None);
    assert_eq!(sent(Enter, ALT, false), None);
    assert_eq!(sent(PageUp, NONE, true), None);
}

#[test]
fn ctrl_space_sends_nul_and_a_plain_space_stays_for_the_table() {
    assert_eq!(named(Space, CTRL, false, false), Some(vec![0]));
    assert_eq!(named(Space, NONE, false, false), None);
}

fn alt(n: winit::keyboard::NamedKey, m: Mods) -> Option<String> {
    named(n, m, false, true).map(|b| String::from_utf8(b).unwrap())
}

/// A full-screen program gets xterm's modified navigation keys, so Ctrl+←
/// word motion, Shift+arrow selection and Ctrl+PgUp tab switching reach
/// vim, micro and helix. The prompt test above keeps the plain keys.
#[test]
fn the_alternate_screen_gets_modified_navigation_keys() {
    let both = Mods {
        shift: true,
        ..CTRL
    };
    assert_eq!(alt(ArrowLeft, CTRL).as_deref(), Some("\x1b[1;5D"));
    assert_eq!(alt(ArrowRight, SHIFT).as_deref(), Some("\x1b[1;2C"));
    assert_eq!(alt(ArrowUp, both).as_deref(), Some("\x1b[1;6A"));
    assert_eq!(alt(Home, SHIFT).as_deref(), Some("\x1b[1;2H"));
    assert_eq!(alt(End, CTRL).as_deref(), Some("\x1b[1;5F"));
    assert_eq!(alt(PageUp, CTRL).as_deref(), Some("\x1b[5;5~"));
    assert_eq!(alt(PageDown, CTRL).as_deref(), Some("\x1b[6;5~"));
    assert_eq!(alt(Delete, CTRL).as_deref(), Some("\x1b[3;5~"));
    assert_eq!(alt(Insert, SHIFT).as_deref(), Some("\x1b[2;2~"));
}

/// The alternate screen leaves the unmodified keys, Option's word motion and
/// Shift+Tab / Shift+Enter to their old answers.
#[test]
fn the_alternate_screen_keeps_plain_keys_and_option_words() {
    assert_eq!(alt(ArrowLeft, NONE), None);
    assert_eq!(alt(PageUp, NONE), None);
    assert_eq!(alt(ArrowLeft, ALT).as_deref(), Some("\x1bb"));
    assert_eq!(alt(Tab, SHIFT), None);
    assert_eq!(alt(Enter, SHIFT), None);
    assert_eq!(named(ArrowUp, NONE, true, true), Some(b"\x1bOA".to_vec()));
}

/// At the prompt Ctrl+PgUp and Shift+Delete stay plain (bare zsh eats the
/// modified forms and the key after them).
#[test]
fn the_prompt_keeps_plain_navigation_keys() {
    assert_eq!(sent(PageUp, CTRL, false), None);
    assert_eq!(sent(Delete, SHIFT, false), None);
    assert_eq!(sent(End, CTRL, false), None);
}

/// The split follows the program's own screen switch: vim entering the
/// alternate screen (`?1049h`) turns the modified keys on, and leaving it
/// gives the shell its plain keys back.
#[test]
fn the_screen_a_program_switches_to_picks_the_encoding() {
    use crew_term::{GridSize, HeadlessTerm, TermModel};
    let mut t = HeadlessTerm::new(GridSize { cols: 80, rows: 24 });
    let ctrl_left = |m: crew_term::InputModes| named(ArrowLeft, CTRL, m.app_cursor, m.alt_screen);
    assert_eq!(ctrl_left(t.input_modes()), None);
    t.feed(b"\x1b[?1049h");
    assert_eq!(ctrl_left(t.input_modes()), Some(b"\x1b[1;5D".to_vec()));
    t.feed(b"\x1b[?1049l");
    assert_eq!(ctrl_left(t.input_modes()), None);
}

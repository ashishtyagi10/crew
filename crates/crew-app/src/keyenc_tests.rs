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
    named(n, m, app).map(|b| String::from_utf8(b).unwrap())
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

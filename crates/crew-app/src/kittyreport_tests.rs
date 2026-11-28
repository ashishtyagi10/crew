//! Flags 2, 4, 8 and 16 byte by byte, against the examples in the kitty
//! keyboard protocol spec.
use winit::keyboard::{Key, KeyCode, NamedKey};

use super::{encode, Ev, Kind, Kind::*, ALL_KEYS as ALL, ALTERNATES as ALT, EVENTS as EVT};
use crate::keyenc::Mods;

pub(super) const NONE: Mods = Mods {
    shift: false,
    alt: false,
    ctrl: false,
};
pub(super) const SHIFT: Mods = Mods {
    shift: true,
    ..NONE
};
pub(super) const CTRL: Mods = Mods { ctrl: true, ..NONE };
pub(super) const ALTM: Mods = Mods { alt: true, ..NONE };
pub(super) const CTRL_SHIFT: Mods = Mods {
    ctrl: true,
    shift: true,
    ..NONE
};

pub(super) fn out(b: Option<Vec<u8>>) -> Option<String> {
    b.map(|b| String::from_utf8(b).unwrap())
}

/// A text key: `own` what it types, `bare` its keycap, `code` its physical key.
pub(super) fn chr(
    flags: u8,
    own: &str,
    bare: &str,
    code: KeyCode,
    m: Mods,
    kind: Kind,
) -> Option<String> {
    let (l, b) = (Key::Character(own.into()), Key::Character(bare.into()));
    let e = Ev {
        logical: &l,
        bare: &b,
        text: Some(own),
        code: Some(code),
        numpad: false,
        kind,
    };
    out(encode(flags, &e, m, false, false))
}

pub(super) fn named(flags: u8, n: NamedKey, code: KeyCode, m: Mods, kind: Kind) -> Option<String> {
    let k = Key::Named(n);
    let e = Ev {
        logical: &k,
        bare: &k,
        text: None,
        code: Some(code),
        numpad: false,
        kind,
    };
    out(encode(flags, &e, m, false, false))
}

pub(super) fn a(flags: u8, m: Mods, kind: Kind) -> Option<String> {
    let own = if m.shift { "A" } else { "a" };
    chr(flags, own, "a", KeyCode::KeyA, m, kind)
}

#[test]
fn flag_8_sends_plain_text_keys_as_escape_codes() {
    assert_eq!(a(ALL, NONE, Press).as_deref(), Some("\x1b[97u"));
    assert_eq!(a(ALL, SHIFT, Press).as_deref(), Some("\x1b[97;2u"));
    assert_eq!(a(ALL, CTRL_SHIFT, Press).as_deref(), Some("\x1b[97;6u"));
    assert_eq!(a(ALL, ALTM, Press).as_deref(), Some("\x1b[97;3u"));
    // Without flag 8 (and without 1) a typed letter stays text.
    assert_eq!(a(EVT, NONE, Press), None);
    assert_eq!(a(EVT | ALT, SHIFT, Press), None);
}

#[test]
fn flag_8_sends_enter_tab_and_backspace_as_escape_codes() {
    use NamedKey::*;
    let enter = |f, k| named(f, Enter, KeyCode::Enter, NONE, k);
    assert_eq!(enter(ALL, Press).as_deref(), Some("\x1b[13u"));
    assert_eq!(
        named(ALL, Tab, KeyCode::Tab, NONE, Press).as_deref(),
        Some("\x1b[9u")
    );
    assert_eq!(
        named(ALL, Backspace, KeyCode::Backspace, NONE, Press).as_deref(),
        Some("\x1b[127u")
    );
    assert_eq!(
        named(ALL, Escape, KeyCode::Escape, NONE, Press).as_deref(),
        Some("\x1b[27u")
    );
    // Flag 1 alone leaves them legacy; flag 2 alone never reports them.
    assert_eq!(enter(1, Press), None);
    assert_eq!(enter(EVT, Press), None);
    assert_eq!(enter(EVT, Release), None, "no release without flag 8");
    assert_eq!(enter(ALL | EVT, Release).as_deref(), Some("\x1b[13;1:3u"));
}

#[test]
fn flag_2_marks_repeats_and_releases() {
    let f = ALL | EVT;
    assert_eq!(a(f, NONE, Press).as_deref(), Some("\x1b[97u"));
    assert_eq!(a(f, NONE, Repeat).as_deref(), Some("\x1b[97;1:2u"));
    assert_eq!(a(f, NONE, Release).as_deref(), Some("\x1b[97;1:3u"));
    assert_eq!(a(f, SHIFT, Release).as_deref(), Some("\x1b[97;2:3u"));
    assert_eq!(a(f, CTRL, Repeat).as_deref(), Some("\x1b[97;5:2u"));
}

/// A release or repeat means nothing without flag 2: a repeat is a press.
#[test]
fn without_flag_2_releases_are_dropped_and_repeats_are_presses() {
    assert_eq!(a(ALL, NONE, Release), None);
    assert_eq!(a(ALL, NONE, Repeat).as_deref(), Some("\x1b[97u"));
    let up = |f, k| named(f, NamedKey::ArrowUp, KeyCode::ArrowUp, NONE, k);
    assert_eq!(up(ALL, Release), None);
    assert_eq!(up(ALL, Repeat), None, "legacy arrow");
}

#[test]
fn flag_2_reports_the_keys_that_are_escape_codes_anyway() {
    use NamedKey::*;
    let up = |k, m| named(EVT, ArrowUp, KeyCode::ArrowUp, m, k);
    assert_eq!(up(Press, NONE), None, "a plain press stays legacy");
    assert_eq!(up(Release, NONE).as_deref(), Some("\x1b[1;1:3A"));
    assert_eq!(up(Repeat, SHIFT).as_deref(), Some("\x1b[1;2:2A"));
    assert_eq!(up(Press, SHIFT).as_deref(), Some("\x1b[1;2A"));
    let f5 = |k| named(EVT, F5, KeyCode::F5, NONE, k);
    assert_eq!(f5(Release).as_deref(), Some("\x1b[15;1:3~"));
    let del = named(EVT, Delete, KeyCode::Delete, CTRL, Release);
    assert_eq!(del.as_deref(), Some("\x1b[3;5:3~"));
    let f1 = named(EVT, F1, KeyCode::F1, NONE, Release);
    assert_eq!(f1.as_deref(), Some("\x1b[1;1:3P"));
    let f3 = named(EVT, F3, KeyCode::F3, NONE, Release);
    assert_eq!(f3.as_deref(), Some("\x1b[13;1:3~"));
    // Esc's press is a legacy byte unless flag 1 or 8 (its release: flag 2).
    assert_eq!(named(EVT, Escape, KeyCode::Escape, NONE, Press), None);
    let esc = named(1 | EVT, Escape, KeyCode::Escape, NONE, Release);
    assert_eq!(esc.as_deref(), Some("\x1b[27;1:3u"));
}

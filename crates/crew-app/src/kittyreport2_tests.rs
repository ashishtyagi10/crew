//! Flags 4 and 16, composed text, the keypad and modifier keys: the other
//! half of `kittyreport_tests`, with the same helpers.
use winit::keyboard::{Key, KeyCode, NamedKey};

use super::{encode, Ev, Kind::*, ALL_KEYS as ALL, ALTERNATES as ALT, EVENTS as EVT, TEXT};
use crate::keyenc::Mods;

use super::tests::{a, chr, named, out, ALTM, CTRL, CTRL_SHIFT, NONE, SHIFT};

#[test]
fn flag_4_adds_the_shifted_and_base_layout_keys() {
    let f = ALL | ALT;
    assert_eq!(a(f, SHIFT, Press).as_deref(), Some("\x1b[97:65;2u"));
    assert_eq!(a(f, NONE, Press).as_deref(), Some("\x1b[97u"));
    // A shifted symbol: Shift+1 types `!`.
    let bang = chr(f, "!", "1", KeyCode::Digit1, SHIFT, Press);
    assert_eq!(bang.as_deref(), Some("\x1b[49:33;2u"));
    // Ctrl+с on a Russian layout: the key's own code, no shifted, base `c`.
    let ru = chr(f, "с", "с", KeyCode::KeyC, CTRL, Press);
    assert_eq!(ru.as_deref(), Some("\x1b[1089::99;5u"));
    // Both at once: Shift+Ctrl+с.
    let both = chr(f, "С", "с", KeyCode::KeyC, CTRL_SHIFT, Press);
    assert_eq!(both.as_deref(), Some("\x1b[1089:1057:99;6u"));
}

#[test]
fn flag_16_adds_the_text_a_press_typed() {
    let f = ALL | TEXT;
    assert_eq!(a(f, NONE, Press).as_deref(), Some("\x1b[97;;97u"));
    assert_eq!(a(f, SHIFT, Press).as_deref(), Some("\x1b[97;2;65u"));
    assert_eq!(a(f, NONE, Release), None);
    assert_eq!(
        a(f | EVT, NONE, Release).as_deref(),
        Some("\x1b[97;1:3u"),
        "never on a release"
    );
    assert_eq!(a(f | EVT, NONE, Repeat).as_deref(), Some("\x1b[97;1:2;97u"));
    assert_eq!(
        a(f, CTRL, Press).as_deref(),
        Some("\x1b[97;5u"),
        "not a chord"
    );
    let all = ALL | TEXT | ALT | EVT;
    assert_eq!(a(all, SHIFT, Press).as_deref(), Some("\x1b[97:65;2;65u"));
}

/// The text a layout composed is the key's own text only to a program that
/// asked for text as codepoints; for the rest it types as before.
#[test]
fn composed_text_follows_flag_16() {
    let e = |text: &'static str, f, m, mac| {
        let (l, b) = (Key::Character(text.into()), Key::Character("a".into()));
        let ev = Ev {
            logical: &l,
            bare: &b,
            text: Some(text),
            code: Some(KeyCode::KeyA),
            numpad: false,
            kind: Press,
        };
        out(encode(f, &ev, m, mac, false))
    };
    // A Mac's Option+a types å: plain text unless 8 and 16 both ask.
    assert_eq!(e("å", ALL, ALTM, true), None);
    assert_eq!(
        e("å", ALL | TEXT, ALTM, true).as_deref(),
        Some("\x1b[97;;229u")
    );
    // AltGr off a Mac (Ctrl+Alt) likewise.
    let altgr = Mods {
        ctrl: true,
        alt: true,
        ..NONE
    };
    assert_eq!(e("å", ALL, altgr, false), None);
    // Option as Meta is a real Alt chord.
    let (l, b) = (Key::Character("å".into()), Key::Character("a".into()));
    let ev = Ev {
        logical: &l,
        bare: &b,
        text: Some("å"),
        code: None,
        numpad: false,
        kind: Press,
    };
    assert_eq!(
        out(encode(ALL, &ev, ALTM, true, true)).as_deref(),
        Some("\x1b[97;3u")
    );
}

#[test]
fn the_keypad_and_modifier_keys_are_reported_under_flag_8() {
    let kp = |f, text: Option<&'static str>, code, k| {
        let l = Key::Character(text.unwrap_or("5").into());
        let ev = Ev {
            logical: &l,
            bare: &l,
            text,
            code: Some(code),
            numpad: true,
            kind: k,
        };
        out(encode(f, &ev, NONE, false, false))
    };
    assert_eq!(
        kp(ALL, Some("5"), KeyCode::Numpad5, Press).as_deref(),
        Some("\x1b[57404u")
    );
    assert_eq!(kp(1, Some("5"), KeyCode::Numpad5, Press), None);
    let e = kp(ALL | TEXT, Some("5"), KeyCode::Numpad5, Press);
    assert_eq!(e.as_deref(), Some("\x1b[57404;;53u"));
    let k = Key::Named(NamedKey::Enter);
    let ev = Ev {
        logical: &k,
        bare: &k,
        text: None,
        code: Some(KeyCode::NumpadEnter),
        numpad: true,
        kind: Press,
    };
    let enter = out(encode(ALL, &ev, NONE, false, false));
    assert_eq!(enter.as_deref(), Some("\x1b[57414u"));
    let shift = |m, k| named(ALL | EVT, NamedKey::Shift, KeyCode::ShiftLeft, m, k);
    assert_eq!(shift(NONE, Press).as_deref(), Some("\x1b[57441;2u"));
    assert_eq!(shift(SHIFT, Release).as_deref(), Some("\x1b[57441;1:3u"));
    let ctrl = named(ALL, NamedKey::Control, KeyCode::ControlLeft, NONE, Press);
    assert_eq!(ctrl.as_deref(), Some("\x1b[57442;5u"));
    let none = named(1 | EVT, NamedKey::Shift, KeyCode::ShiftLeft, NONE, Press);
    assert_eq!(none, None, "modifier keys need flag 8");
}

#[test]
fn dead_keys_and_unnamed_keys_send_nothing() {
    let dead = Key::Dead(Some('´'));
    let e = Ev {
        logical: &dead,
        bare: &dead,
        text: None,
        code: Some(KeyCode::KeyE),
        numpad: false,
        kind: Press,
    };
    assert_eq!(encode(ALL | TEXT, &e, ALTM, true, false), None);
}

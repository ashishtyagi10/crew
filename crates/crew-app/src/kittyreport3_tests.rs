//! Review fixes: releases of Enter/Tab/Backspace, Esc under flag 2 alone, and
//! control characters never offered as a shifted key.
use winit::keyboard::{KeyCode, NamedKey};

use super::tests::{chr, named, NONE, SHIFT};
use super::{Kind::*, ALL_KEYS as ALL, ALTERNATES as ALT, EVENTS as EVT};

#[test]
fn enter_tab_backspace_releases_need_flag_8() {
    // Flags 1|2: a Shift+Enter press is an escape code, its release is not.
    let f = 1 | EVT;
    let enter = |k| named(f, NamedKey::Enter, KeyCode::Enter, SHIFT, k);
    assert_eq!(enter(Press).as_deref(), Some("\x1b[13;2u"));
    assert_eq!(enter(Release), None);
    let tab = named(f, NamedKey::Tab, KeyCode::Tab, SHIFT, Release);
    assert_eq!(tab, None);
    let bs = named(f, NamedKey::Backspace, KeyCode::Backspace, SHIFT, Release);
    assert_eq!(bs, None);
    // With 8 they are reported like any key.
    let all = named(f | ALL, NamedKey::Enter, KeyCode::Enter, SHIFT, Release);
    assert_eq!(all.as_deref(), Some("\x1b[13;2:3u"));
}

#[test]
fn escape_release_is_reported_under_flag_2_alone() {
    let esc = |k| named(EVT, NamedKey::Escape, KeyCode::Escape, NONE, k);
    assert_eq!(esc(Release).as_deref(), Some("\x1b[27;1:3u"));
    assert_eq!(esc(Press), None, "the press stays the legacy byte");
    assert_eq!(
        named(0, NamedKey::Escape, KeyCode::Escape, NONE, Release),
        None
    );
}

#[test]
fn a_control_character_is_not_a_shifted_key() {
    // macOS may hand Ctrl+Shift+a the logical key `\x01`.
    let f = ALL | ALT;
    let r = chr(f, "\u{1}", "a", KeyCode::KeyA, SHIFT, Press);
    assert_eq!(
        r.as_deref(),
        Some("\x1b[97;2u"),
        "no code 1 in the shifted slot"
    );
}

//! The kitty keyboard protocol's flag 1, key by key (see the module doc).
use winit::keyboard::{Key, NamedKey, NamedKey::*};

use super::{armed, encode, Mods};

const NONE: Mods = Mods {
    shift: false,
    alt: false,
    ctrl: false,
};
const SHIFT: Mods = Mods {
    shift: true,
    ..NONE
};
const ALT: Mods = Mods { alt: true, ..NONE };
const CTRL: Mods = Mods { ctrl: true, ..NONE };
const CTRL_SHIFT: Mods = Mods {
    ctrl: true,
    shift: true,
    ..NONE
};

/// What a named key sends off a Mac, the flag on.
fn key(n: NamedKey, m: Mods) -> Option<String> {
    let k = Key::Named(n);
    let b = encode(&k, &k, None, m, true, false, false)?;
    Some(String::from_utf8(b).unwrap())
}

/// What a text key sends: `own` the logical key, `bare` the keycap's,
/// `typed` its text; `mac` and `meta` (Option as Meta).
fn chr(own: &str, bare: &str, typed: &str, m: Mods, mac: bool, meta: bool) -> Option<String> {
    let (own, bare) = (Key::Character(own.into()), Key::Character(bare.into()));
    let b = encode(&own, &bare, Some(typed), m, true, mac, meta)?;
    Some(String::from_utf8(b).unwrap())
}

#[test]
fn esc_is_its_own_key() {
    assert_eq!(key(Escape, NONE).as_deref(), Some("\x1b[27u"));
    assert_eq!(key(Escape, SHIFT).as_deref(), Some("\x1b[27;2u"));
}

#[test]
fn ctrl_and_alt_chords_name_the_unshifted_key() {
    // Ctrl+I is not Tab, Ctrl+M is not Enter.
    assert_eq!(
        chr("i", "i", "\t", CTRL, false, false).as_deref(),
        Some("\x1b[105;5u")
    );
    assert_eq!(
        chr("m", "m", "\r", CTRL, false, false).as_deref(),
        Some("\x1b[109;5u")
    );
    // Ctrl+Shift+A is 97, never 65, and not Ctrl+A.
    assert_eq!(
        chr("A", "a", "\u{1}", CTRL_SHIFT, false, false).as_deref(),
        Some("\x1b[97;6u")
    );
    // Even where the platform hands back the keycap capitalised (Caps Lock).
    let caps = chr("A", "A", "\u{1}", CTRL_SHIFT, false, false);
    assert_eq!(caps.as_deref(), Some("\x1b[97;6u"));
    assert_eq!(
        chr("!", "1", "!", CTRL_SHIFT, false, false).as_deref(),
        Some("\x1b[49;6u")
    );
    assert_eq!(
        chr("x", "x", "x", ALT, false, false).as_deref(),
        Some("\x1b[120;3u")
    );
    assert_eq!(
        chr("[", "[", "\u{1b}", CTRL, false, false).as_deref(),
        Some("\x1b[91;5u")
    );
    assert_eq!(key(Space, CTRL).as_deref(), Some("\x1b[32;5u"));
}

#[test]
fn plain_and_shifted_text_stays_text() {
    assert_eq!(chr("a", "a", "a", NONE, false, false), None);
    assert_eq!(chr("A", "a", "A", SHIFT, false, false), None);
    assert_eq!(key(Space, NONE), None);
    assert_eq!(key(Space, SHIFT), None);
}

#[test]
fn enter_tab_and_backspace_change_only_with_a_modifier() {
    for n in [Enter, Tab, Backspace, ArrowUp, Home, PageDown, F1, F5] {
        assert_eq!(key(n, NONE), None, "{n:?} keeps its legacy bytes");
    }
    assert_eq!(key(Enter, SHIFT).as_deref(), Some("\x1b[13;2u"));
    assert_eq!(key(Enter, ALT).as_deref(), Some("\x1b[13;3u"));
    assert_eq!(key(Tab, SHIFT).as_deref(), Some("\x1b[9;2u"));
    assert_eq!(key(Backspace, CTRL).as_deref(), Some("\x1b[127;5u"));
}

#[test]
fn modified_navigation_and_function_keys_take_xterms_forms() {
    assert_eq!(key(ArrowLeft, ALT).as_deref(), Some("\x1b[1;3D"));
    assert_eq!(key(ArrowUp, CTRL_SHIFT).as_deref(), Some("\x1b[1;6A"));
    assert_eq!(key(End, SHIFT).as_deref(), Some("\x1b[1;2F"));
    assert_eq!(key(Delete, CTRL).as_deref(), Some("\x1b[3;5~"));
    assert_eq!(key(PageUp, SHIFT).as_deref(), Some("\x1b[5;2~"));
    assert_eq!(key(F1, CTRL).as_deref(), Some("\x1b[1;5P"));
    assert_eq!(
        key(F3, SHIFT).as_deref(),
        Some("\x1b[13;2~"),
        "CSI R is a cursor report"
    );
    assert_eq!(key(F12, ALT).as_deref(), Some("\x1b[24;3~"));
}

/// A Mac's Option types (Option+e is the acute accent) unless it is Meta;
/// off a Mac, Ctrl+Alt that typed a different character is Windows' AltGr.
#[test]
fn typing_alt_keys_still_type() {
    assert_eq!(chr("´", "e", "´", ALT, true, false), None);
    assert_eq!(
        chr("´", "e", "´", ALT, true, true).as_deref(),
        Some("\x1b[101;3u")
    );
    let ctrl_alt = Mods {
        ctrl: true,
        alt: true,
        ..NONE
    };
    assert_eq!(chr("@", "q", "@", ctrl_alt, false, false), None);
    assert_eq!(
        chr("q", "q", "q", ctrl_alt, false, false).as_deref(),
        Some("\x1b[113;7u")
    );
}

/// The flag off, nothing here answers: every key takes the legacy path
/// unchanged (`keyenc`, `altmeta`, `session`).
#[test]
fn without_the_flag_nothing_changes() {
    let esc = Key::Named(Escape);
    assert_eq!(encode(&esc, &esc, None, NONE, false, false, false), None);
    let (i, bare) = (Key::Character("i".into()), Key::Character("i".into()));
    assert_eq!(
        encode(&i, &bare, Some("\t"), CTRL, false, false, false),
        None
    );
}

/// A Mac's Option is a typing key unless it is Meta: Option+Space types
/// (a no-break space) like Option+letter, and Ctrl still counts.
#[test]
fn option_space_follows_the_mac_option_rule() {
    let sp = |m, mac, meta| {
        let k = Key::Named(Space);
        encode(&k, &k, None, m, true, mac, meta).map(|b| String::from_utf8(b).unwrap())
    };
    assert_eq!(sp(ALT, true, false), None, "Option types");
    assert_eq!(sp(ALT, true, true).as_deref(), Some("\x1b[32;3u"));
    assert_eq!(sp(ALT, false, false).as_deref(), Some("\x1b[32;3u"));
    let ctrl_alt = Mods { ctrl: true, ..ALT };
    assert_eq!(sp(ctrl_alt, true, false).as_deref(), Some("\x1b[32;5u"));
}

/// Broadcast writes one key's bytes to every terminal pane: they are the
/// legacy ones, whatever the focused program asked for.
#[test]
fn broadcast_keys_are_never_kitty_form() {
    assert!(armed(true, false));
    assert!(!armed(true, true), "a zsh beside nvim must read Ctrl+C");
    assert!(!armed(false, false) && !armed(false, true));
}

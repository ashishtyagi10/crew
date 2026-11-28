//! The navigation keys off a Mac, and a Mac's own, which must not move.
use super::*;
use winit::keyboard::NamedKey;

const ALT: ModifiersState = ModifiersState::ALT;
const CTRL: ModifiersState = ModifiersState::CONTROL;
const NONE: ModifiersState = ModifiersState::empty();

fn named(n: NamedKey, mods: ModifiersState, mac: bool) -> Option<WinChord> {
    nav_chord(&Key::Named(n), None, mods, mac)
}

fn digit(n: u8, mods: ModifiersState, mac: bool) -> Option<WinChord> {
    let codes = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
        KeyCode::Digit9,
    ];
    let k = Key::Character(n.to_string().into());
    nav_chord(&k, Some(codes[usize::from(n) - 1]), mods, mac)
}

#[test]
fn off_a_mac_the_navigation_keys_are_two_keys() {
    use NamedKey::*;
    assert_eq!(named(PageUp, CTRL, false), Some(WinChord::Tab(-1)));
    assert_eq!(named(PageDown, CTRL, false), Some(WinChord::Tab(1)));
    assert_eq!(named(PageUp, ALT, false), Some(WinChord::CmdJump(true)));
    assert_eq!(named(PageDown, ALT, false), Some(WinChord::CmdJump(false)));
    assert_eq!(named(Home, ALT, false), Some(WinChord::Screen(-1)));
    assert_eq!(named(End, ALT, false), Some(WinChord::Screen(1)));
    assert_eq!(named(F11, NONE, false), Some(WinChord::Maximize));
    for n in 1..=9u8 {
        let want = Some(WinChord::TabN(usize::from(n)));
        assert_eq!(digit(n, CTRL, false), want, "Ctrl+{n}");
    }
}

/// Alt+1…9 stays pane N (`cmd_chord`'s), never a tab.
#[test]
fn off_a_mac_alt_digits_are_not_navigation() {
    for n in 1..=9u8 {
        assert_eq!(digit(n, ALT, false), None, "Alt+{n}");
    }
}

/// The old Ctrl+Alt chords are AltGr now: they type, never navigate.
#[test]
fn off_a_mac_ctrl_alt_is_no_navigation() {
    use NamedKey::*;
    let altgr = CTRL | ALT;
    for k in [
        PageUp, PageDown, Home, End, ArrowLeft, ArrowRight, ArrowUp, ArrowDown,
    ] {
        assert_eq!(named(k, altgr, false), None, "AltGr+{k:?}");
    }
    for n in 1..=9u8 {
        assert_eq!(digit(n, altgr, false), None, "AltGr+{n}");
    }
    assert!(!crate::ctrlchord::screen_held(altgr, false));
}

/// Keys crew leaves alone reach the pane: other modifiers, plain keys.
#[test]
fn off_a_mac_other_navigation_keys_go_to_the_pane() {
    use NamedKey::*;
    let shift = ModifiersState::SHIFT;
    assert_eq!(named(PageUp, NONE, false), None);
    assert_eq!(named(PageUp, shift, false), None, "Shift+PgUp scrolls");
    assert_eq!(named(Home, NONE, false), None);
    assert_eq!(named(Home, CTRL, false), None, "Ctrl+Home is the shell's");
    assert_eq!(named(End, CTRL, false), None);
    assert_eq!(named(PageUp, CTRL | shift, false), None);
    assert_eq!(named(PageUp, ALT | shift, false), None);
    assert_eq!(named(F11, ALT, false), None);
    assert_eq!(named(F11, CTRL, false), None);
    assert_eq!(named(ArrowLeft, CTRL, false), None, "word jump");
    assert_eq!(digit(1, NONE, false), None);
}

/// On a Mac none of these keys is crew's: they were the pane's before.
#[test]
fn on_a_mac_these_keys_are_unchanged() {
    use NamedKey::*;
    for mods in [NONE, CTRL, ALT, CTRL | ALT, ModifiersState::SUPER] {
        for k in [PageUp, PageDown, Home, End, F11] {
            assert_eq!(named(k, mods, true), None, "{mods:?} {k:?}");
        }
        assert_eq!(digit(1, mods, true), None);
    }
    use crate::ctrlchord::{cmd_held, screen_held};
    let sa = ModifiersState::SUPER | ALT;
    assert!(screen_held(sa, true), "Cmd+Opt+arrows still step screens");
    assert!(cmd_held(ModifiersState::SUPER, true));
    assert!(!cmd_held(ALT, true), "Option is not Cmd");
}

#[test]
fn the_crew_key_is_alt_but_not_altgr_off_a_mac() {
    use crate::ctrlchord::cmd_held;
    assert!(cmd_held(ALT, false));
    assert!(!cmd_held(CTRL | ALT, false));
    assert!(!cmd_held(ModifiersState::SUPER, false));
    assert!(!cmd_held(CTRL, false));
}

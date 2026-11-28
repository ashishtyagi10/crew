//! Which key events reach the kitty encoders, and the releases owed when
//! focus leaves a pane.
use super::{admit, take_stale, Held};
use crate::keyenc::Mods;
use winit::keyboard::{Key, KeyCode::*, NamedKey};

fn held(pane: u64, flags: u8, key: NamedKey) -> Held {
    Held {
        pane,
        flags,
        logical: Key::Named(key),
        bare: Key::Named(key),
        numpad: false,
        mods: Mods::default(),
        mac: false,
        meta: false,
    }
}

fn letter(pane: u64) -> Held {
    let a = Key::Character("a".into());
    Held {
        logical: a.clone(),
        bare: a,
        ..held(pane, 10, NamedKey::Enter)
    }
}

#[test]
fn a_release_follows_only_a_press_the_program_saw() {
    assert!(admit(10, true, Some(KeyA), || letter(1)));
    assert!(admit(10, false, Some(KeyA), || letter(1)), "its release");
    assert!(!admit(10, false, Some(KeyA), || letter(1)), "once");
    // Cmd+B was crew's: no press reached the program, so no release goes.
    assert!(!admit(10, false, Some(KeyB), || letter(1)));
    assert!(!admit(10, false, None, || letter(1)), "no state for none");
}

#[test]
fn flags_off_forget_every_key_held() {
    assert!(admit(2, true, Some(KeyA), || letter(1)));
    assert!(admit(0, true, Some(KeyB), || letter(1)), "a press goes on");
    assert!(!admit(2, false, Some(KeyA), || letter(1)), "flags popped");
}

#[test]
fn focus_leaving_a_pane_owes_it_the_release_of_a_held_shift() {
    // Shift pressed in pane 7 under flags 8|2; focus moves to pane 9.
    assert!(admit(10, true, Some(ShiftLeft), || held(
        7,
        10,
        NamedKey::Shift
    )));
    // The pane focus is on keeps its keys (and nothing goes to it).
    assert!(take_stale(|_| true, |p| p == 7).is_empty());
    let owed = take_stale(|_| true, |p| p == 9);
    assert_eq!(owed, vec![(7, b"\x1b[57441;1:3u".to_vec())]);
    // Once owed it is forgotten: the real release no longer goes on, and a
    // second drain owes nothing.
    assert!(!admit(10, false, Some(ShiftLeft), || letter(9)));
    assert!(take_stale(|_| true, |_| false).is_empty());
}

#[test]
fn a_window_blur_owes_every_pane_and_leaves_other_windows_alone() {
    assert!(admit(10, true, Some(ShiftLeft), || held(
        7,
        10,
        NamedKey::Shift
    )));
    assert!(admit(10, true, Some(ControlLeft), || held(
        8,
        10,
        NamedKey::Control
    )));
    // Only pane 7 is this window's; blur owes it everything, pane 8 stays.
    let owed = take_stale(|p| p == 7, |_| false);
    assert_eq!(owed.len(), 1);
    assert_eq!(owed[0].0, 7);
    assert!(
        admit(10, false, Some(ControlLeft), || letter(8)),
        "still held"
    );
}

#[test]
fn a_program_without_event_types_is_owed_nothing() {
    assert!(admit(8, true, Some(ShiftLeft), || held(
        7,
        8,
        NamedKey::Shift
    )));
    assert!(take_stale(|_| true, |_| false).is_empty());
}

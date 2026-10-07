use super::*;
use winit::keyboard::ModifiersState;

#[test]
fn is_compact_chord_matches_ctrl_o_only() {
    assert!(is_compact_chord(
        &Key::Character("o".into()),
        ModifiersState::CONTROL
    ));
    // Case-insensitive, matching how Ctrl+Shift+M's own match is written.
    assert!(is_compact_chord(
        &Key::Character("O".into()),
        ModifiersState::CONTROL
    ));
}

#[test]
fn is_compact_chord_requires_control() {
    assert!(!is_compact_chord(
        &Key::Character("o".into()),
        ModifiersState::empty()
    ));
}

#[test]
fn is_compact_chord_rejects_other_letters() {
    assert!(!is_compact_chord(
        &Key::Character("k".into()),
        ModifiersState::CONTROL
    ));
}

#[test]
fn arrow_dir_maps_the_four_arrows_and_nothing_else() {
    use crate::panedir::Dir;
    assert_eq!(arrow_dir(&Key::Named(NamedKey::ArrowLeft)), Some(Dir::Left));
    assert_eq!(
        arrow_dir(&Key::Named(NamedKey::ArrowRight)),
        Some(Dir::Right)
    );
    assert_eq!(arrow_dir(&Key::Named(NamedKey::ArrowUp)), Some(Dir::Up));
    assert_eq!(arrow_dir(&Key::Named(NamedKey::ArrowDown)), Some(Dir::Down));
    assert_eq!(arrow_dir(&Key::Named(NamedKey::Enter)), None);
    assert_eq!(arrow_dir(&Key::Character("k".into())), None);
}

#[test]
fn is_compact_chord_rejects_named_keys() {
    assert!(!is_compact_chord(
        &Key::Named(NamedKey::Escape),
        ModifiersState::CONTROL
    ));
}

#[test]
fn off_a_mac_ctrl_shift_stands_in_for_cmd() {
    let cs = ModifiersState::CONTROL | ModifiersState::SHIFT;
    let ch = |s: &str| Key::Character(s.into());
    assert_eq!(stand_in(&ch("i"), cs, false).as_deref(), Some("i"));
    assert_eq!(stand_in(&ch("1"), cs, false).as_deref(), Some("1"));
    assert_eq!(stand_in(&ch("i"), cs, true), None, "a Mac has its Cmd key");
    assert_eq!(stand_in(&ch("i"), ModifiersState::CONTROL, false), None);
    let alt = cs | ModifiersState::ALT;
    assert_eq!(stand_in(&ch("i"), alt, false), None, "AltGr is Ctrl+Alt");
    // Ctrl+Shift chords crew already means something by keep that meaning.
    for k in ["l", "g", "f", "m"] {
        assert_eq!(stand_in(&ch(k), cs, false), None, "{k}");
    }
    // …and Ctrl+Shift+O is Cmd+O, not the compact view's Ctrl+O.
    assert!(!is_compact_chord(&ch("O"), cs));
}

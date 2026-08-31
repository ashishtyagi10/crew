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

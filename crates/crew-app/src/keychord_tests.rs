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
fn off_a_mac_the_grid_walks_with_alt_and_never_with_altgr() {
    use crate::ctrlchord::grid_held;
    assert!(grid_held(ModifiersState::ALT, false));
    assert!(grid_held(
        ModifiersState::ALT | ModifiersState::SHIFT,
        false
    ));
    assert!(!grid_held(
        ModifiersState::ALT | ModifiersState::CONTROL,
        false
    ));
    assert!(
        !grid_held(ModifiersState::ALT, true),
        "a Mac's Option moves by word"
    );
    assert!(grid_held(ModifiersState::SUPER, true));
}

/// Ctrl+N in Far's command line or a settings field is no chord (off a Mac
/// plain Ctrl never is) — and no letter either: it typed a stray `n`.
#[test]
fn a_ctrl_letter_is_no_text_to_type() {
    let ch = |s: &str| Key::Character(s.into());
    assert!(ctrl_letter(&ch("n"), ModifiersState::CONTROL));
    assert!(ctrl_letter(
        &ch("N"),
        ModifiersState::CONTROL | ModifiersState::SHIFT
    ));
    assert!(
        !ctrl_letter(&ch("n"), ModifiersState::empty()),
        "a plain letter types"
    );
    let altgr = ModifiersState::CONTROL | ModifiersState::ALT;
    assert!(
        !ctrl_letter(&ch("q"), altgr),
        "AltGr+Q is @ on a German layout"
    );
    assert!(!ctrl_letter(&ch("@"), ModifiersState::CONTROL));
}

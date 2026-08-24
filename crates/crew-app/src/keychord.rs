//! Two key *predicates* pulled out of [`crate::keys`]'s dispatch: the arrow
//! that names a grid direction, and the chord that toggles a chat pane's
//! compact view. Both are pure functions over a `winit` key so the mapping is
//! testable without constructing a `KeyEvent`.
use winit::keyboard::{Key, NamedKey};

/// The arrow direction a key names, or `None` for every other key. Kept as a
/// free function (like [`is_compact_chord`]) so the mapping is testable
/// without building a winit `KeyEvent`.
pub(crate) fn arrow_dir(key: &Key) -> Option<crate::panedir::Dir> {
    use crate::panedir::Dir;
    match key {
        Key::Named(NamedKey::ArrowLeft) => Some(Dir::Left),
        Key::Named(NamedKey::ArrowRight) => Some(Dir::Right),
        Key::Named(NamedKey::ArrowUp) => Some(Dir::Up),
        Key::Named(NamedKey::ArrowDown) => Some(Dir::Down),
        _ => None,
    }
}

/// Ctrl+O — the chord that toggles a chat pane's compact transcript view.
/// Extracted as a pure predicate (mirrors `swarmpane::esc_closes`) so the
/// match is testable without constructing a winit `KeyEvent`. Modeled on the
/// Ctrl+Shift+M intercept above: same reach (fires before the input-bar
/// early-return), but with no Shift requirement.
pub(crate) fn is_compact_chord(key: &Key, mods: winit::keyboard::ModifiersState) -> bool {
    mods.control_key() && matches!(key, Key::Character(s) if s.eq_ignore_ascii_case("o"))
}

#[cfg(test)]
mod tests {
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
}

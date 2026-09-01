//! The classifier is a table, so these are the rows a reader would check by hand: the chords
//! that mean something only in a document window, and the ones that must NOT be swallowed.
use super::*;

fn ch(s: &str) -> Key {
    Key::Character(s.into())
}

const CMD: ModifiersState = ModifiersState::SUPER;

#[test]
fn tab_is_its_own_intention_rather_than_two_spaces() {
    // It types spaces only when the caret is not in a table, and only the handler knows that.
    assert_eq!(
        edit_for(&Key::Named(NamedKey::Tab), true, ModifiersState::empty()),
        Some(Edit::Tab)
    );
}

#[test]
fn cmd_k_reaches_for_a_link() {
    assert_eq!(edit_for(&ch("k"), true, CMD), Some(Edit::Link));
    assert_eq!(
        edit_for(&ch("k"), true, ModifiersState::CONTROL),
        Some(Edit::Link),
        "Ctrl is the same chord away from a Mac"
    );
}

#[test]
fn a_bare_k_is_a_letter() {
    // The window is an editor: every unmodified letter has to type itself.
    assert_eq!(
        edit_for(&ch("k"), true, ModifiersState::empty()),
        Some(Edit::Type("k".into()))
    );
}

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
        edit_for(
            &Key::Named(NamedKey::Tab),
            true,
            ModifiersState::empty(),
            20
        ),
        Some(Edit::Tab)
    );
}

#[test]
fn cmd_k_reaches_for_a_link() {
    assert_eq!(edit_for(&ch("k"), true, CMD, 20), Some(Edit::Link));
    assert_eq!(
        edit_for(&ch("k"), true, ModifiersState::CONTROL, 20),
        Some(Edit::Link),
        "Ctrl is the same chord away from a Mac"
    );
}

#[test]
fn a_bare_k_is_a_letter() {
    // The window is an editor: every unmodified letter has to type itself.
    assert_eq!(
        edit_for(&ch("k"), true, ModifiersState::empty(), 20),
        Some(Edit::Type("k".into()))
    );
}

#[test]
fn page_keys_move_the_caret_by_the_windows_height() {
    let none = ModifiersState::empty();
    assert_eq!(
        edit_for(&Key::Named(NamedKey::PageDown), true, none, 23),
        Some(Edit::Move(Step::Page {
            down: true,
            rows: 23
        }))
    );
    assert_eq!(
        edit_for(
            &Key::Named(NamedKey::PageUp),
            true,
            ModifiersState::SHIFT,
            23
        ),
        Some(Edit::Select(Step::Page {
            down: false,
            rows: 23
        })),
        "and Shift drags a selection behind it"
    );
}

#[test]
fn cmd_arrows_are_the_line_and_document_ends() {
    let up = Key::Named(NamedKey::ArrowUp);
    assert_eq!(edit_for(&up, true, CMD, 20), Some(Edit::Move(Step::Top)));
    assert_eq!(
        edit_for(&up, true, ModifiersState::empty(), 20),
        Some(Edit::Move(Step::Up))
    );
    assert_eq!(
        edit_for(&Key::Named(NamedKey::ArrowDown), true, CMD, 20),
        Some(Edit::Move(Step::Bottom))
    );
    assert_eq!(
        edit_for(
            &Key::Named(NamedKey::ArrowLeft),
            true,
            CMD | ModifiersState::SHIFT,
            20
        ),
        Some(Edit::Select(Step::Home))
    );
    assert_eq!(
        edit_for(&Key::Named(NamedKey::ArrowRight), true, CMD, 20),
        Some(Edit::Move(Step::End))
    );
}

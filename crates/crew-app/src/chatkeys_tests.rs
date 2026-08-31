use super::*;

#[test]
fn escape_key_requests_pane_close() {
    assert_eq!(
        chat_key(&Key::Named(NamedKey::Escape), true, false, false),
        ChatInput::Close
    );
}

#[test]
fn a_released_key_is_ignored() {
    // Only key presses act; releases (including Escape) do nothing.
    assert_eq!(
        chat_key(&Key::Named(NamedKey::Escape), false, false, false),
        ChatInput::Ignore
    );
}

#[test]
fn tab_requests_completion() {
    assert_eq!(
        chat_key(&Key::Named(NamedKey::Tab), true, false, false),
        ChatInput::Complete
    );
}

#[test]
fn arrows_classify_for_popup_navigation() {
    assert_eq!(
        chat_key(&Key::Named(NamedKey::ArrowUp), true, false, false),
        ChatInput::Up
    );
    assert_eq!(
        chat_key(&Key::Named(NamedKey::ArrowDown), true, false, false),
        ChatInput::Down
    );
}

#[test]
fn right_arrow_accepts_a_suggestion() {
    assert_eq!(
        chat_key(&Key::Named(NamedKey::ArrowRight), true, false, false),
        ChatInput::Accept
    );
    // Left is still nothing — the composer has no cursor to move.
    assert_eq!(
        chat_key(&Key::Named(NamedKey::ArrowLeft), true, false, false),
        ChatInput::Ignore
    );
}

#[test]
fn shift_enter_inserts_a_newline_instead_of_sending() {
    assert_eq!(
        chat_key(&Key::Named(NamedKey::Enter), true, true, false),
        ChatInput::Newline
    );
    assert_eq!(
        chat_key(&Key::Named(NamedKey::Enter), true, false, false),
        ChatInput::Enter
    );
}

#[test]
fn ctrl_r_is_the_history_search_chord() {
    assert_eq!(
        chat_key(&Key::Character("r".into()), true, false, true),
        ChatInput::HistSearch
    );
    // Case-insensitive, like the `keys.rs` chord intercepts.
    assert_eq!(
        chat_key(&Key::Character("R".into()), true, false, true),
        ChatInput::HistSearch
    );
    // Plain 'r' still types; Ctrl with another letter changes nothing.
    assert_eq!(
        chat_key(&Key::Character("r".into()), true, false, false),
        ChatInput::Char('r')
    );
    assert_eq!(
        chat_key(&Key::Character("x".into()), true, false, true),
        ChatInput::Char('x')
    );
}

#[test]
fn typed_characters_and_edits_are_classified() {
    assert_eq!(
        chat_key(&Key::Character("a".into()), true, false, false),
        ChatInput::Char('a')
    );
    assert_eq!(
        chat_key(&Key::Named(NamedKey::Space), true, false, false),
        ChatInput::Char(' ')
    );
    assert_eq!(
        chat_key(&Key::Named(NamedKey::Enter), true, false, false),
        ChatInput::Enter
    );
    assert_eq!(
        chat_key(&Key::Named(NamedKey::Backspace), true, false, false),
        ChatInput::Backspace
    );
}

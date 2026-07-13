//! Key classification for chat panes. Extracted from `chat.rs` as a pure,
//! testable seam (winit's `KeyEvent` is `#[non_exhaustive]` and hard to build).
use winit::keyboard::{Key, NamedKey};

/// What a key press means to a chat pane.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ChatInput {
    Close,
    Char(char),
    Enter,
    /// Shift+Enter — insert a newline instead of sending.
    Newline,
    Backspace,
    /// Tab — complete the leading @agent / /construct token.
    Complete,
    /// Arrow keys — navigate the @file mention popup when it is open.
    Up,
    Down,
    Ignore,
}

/// An action a chat pane asks the app to take after a key press.
pub(crate) enum ChatAction {
    /// Close this pane (Escape).
    Close,
}

/// Classify a key press for a chat pane. Only presses act; Escape closes.
/// `shift` turns Enter into a newline instead of a send.
pub(crate) fn chat_key(logical: &Key, pressed: bool, shift: bool) -> ChatInput {
    if !pressed {
        return ChatInput::Ignore;
    }
    match logical {
        Key::Named(NamedKey::Escape) => ChatInput::Close,
        Key::Named(NamedKey::Tab) => ChatInput::Complete,
        Key::Named(NamedKey::ArrowUp) => ChatInput::Up,
        Key::Named(NamedKey::ArrowDown) => ChatInput::Down,
        Key::Named(NamedKey::Enter) if shift => ChatInput::Newline,
        Key::Named(NamedKey::Enter) => ChatInput::Enter,
        Key::Named(NamedKey::Backspace) => ChatInput::Backspace,
        Key::Named(NamedKey::Space) => ChatInput::Char(' '),
        Key::Character(s) => s.chars().next().map_or(ChatInput::Ignore, ChatInput::Char),
        _ => ChatInput::Ignore,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_key_requests_pane_close() {
        assert_eq!(
            chat_key(&Key::Named(NamedKey::Escape), true, false),
            ChatInput::Close
        );
    }

    #[test]
    fn a_released_key_is_ignored() {
        // Only key presses act; releases (including Escape) do nothing.
        assert_eq!(
            chat_key(&Key::Named(NamedKey::Escape), false, false),
            ChatInput::Ignore
        );
    }

    #[test]
    fn tab_requests_completion() {
        assert_eq!(
            chat_key(&Key::Named(NamedKey::Tab), true, false),
            ChatInput::Complete
        );
    }

    #[test]
    fn arrows_classify_for_popup_navigation() {
        assert_eq!(
            chat_key(&Key::Named(NamedKey::ArrowUp), true, false),
            ChatInput::Up
        );
        assert_eq!(
            chat_key(&Key::Named(NamedKey::ArrowDown), true, false),
            ChatInput::Down
        );
    }

    #[test]
    fn shift_enter_inserts_a_newline_instead_of_sending() {
        assert_eq!(
            chat_key(&Key::Named(NamedKey::Enter), true, true),
            ChatInput::Newline
        );
        assert_eq!(
            chat_key(&Key::Named(NamedKey::Enter), true, false),
            ChatInput::Enter
        );
    }

    #[test]
    fn typed_characters_and_edits_are_classified() {
        assert_eq!(
            chat_key(&Key::Character("a".into()), true, false),
            ChatInput::Char('a')
        );
        assert_eq!(
            chat_key(&Key::Named(NamedKey::Space), true, false),
            ChatInput::Char(' ')
        );
        assert_eq!(
            chat_key(&Key::Named(NamedKey::Enter), true, false),
            ChatInput::Enter
        );
        assert_eq!(
            chat_key(&Key::Named(NamedKey::Backspace), true, false),
            ChatInput::Backspace
        );
    }
}

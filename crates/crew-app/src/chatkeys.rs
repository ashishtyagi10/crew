//! Key classification for chat panes. Extracted from `chat.rs` as a pure,
//! testable seam (winit's `KeyEvent` is `#[non_exhaustive]` and hard to build).
use winit::keyboard::{Key, NamedKey};

/// What a key press means to a chat pane.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ChatInput {
    Close,
    Char(char),
    Enter,
    Backspace,
    Ignore,
}

/// An action a chat pane asks the app to take after a key press.
pub(crate) enum ChatAction {
    /// Close this pane (Escape).
    Close,
}

/// Classify a key press for a chat pane. Only presses act; Escape closes.
pub(crate) fn chat_key(logical: &Key, pressed: bool) -> ChatInput {
    if !pressed {
        return ChatInput::Ignore;
    }
    match logical {
        Key::Named(NamedKey::Escape) => ChatInput::Close,
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
            chat_key(&Key::Named(NamedKey::Escape), true),
            ChatInput::Close
        );
    }

    #[test]
    fn a_released_key_is_ignored() {
        // Only key presses act; releases (including Escape) do nothing.
        assert_eq!(
            chat_key(&Key::Named(NamedKey::Escape), false),
            ChatInput::Ignore
        );
    }

    #[test]
    fn typed_characters_and_edits_are_classified() {
        assert_eq!(
            chat_key(&Key::Character("a".into()), true),
            ChatInput::Char('a')
        );
        assert_eq!(
            chat_key(&Key::Named(NamedKey::Space), true),
            ChatInput::Char(' ')
        );
        assert_eq!(
            chat_key(&Key::Named(NamedKey::Enter), true),
            ChatInput::Enter
        );
        assert_eq!(
            chat_key(&Key::Named(NamedKey::Backspace), true),
            ChatInput::Backspace
        );
    }
}

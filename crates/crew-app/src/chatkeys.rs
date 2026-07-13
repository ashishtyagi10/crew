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
    /// Ctrl+O — toggle the compact transcript view (see
    /// `ChatPane::compact_view`).
    ToggleCompact,
    Ignore,
}

/// An action a chat pane asks the app to take after a key press.
pub(crate) enum ChatAction {
    /// Close this pane (Escape).
    Close,
}

/// Classify a key press for a chat pane. Only presses act; Escape closes.
/// `shift` turns Enter into a newline instead of a send; `ctrl` turns `o`
/// into the compact-transcript toggle instead of a literal character.
pub(crate) fn chat_key(logical: &Key, pressed: bool, shift: bool, ctrl: bool) -> ChatInput {
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
        Key::Character(s) if ctrl && s.eq_ignore_ascii_case("o") => ChatInput::ToggleCompact,
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

    #[test]
    fn ctrl_o_requests_compact_toggle() {
        assert_eq!(
            chat_key(&Key::Character("o".into()), true, false, true),
            ChatInput::ToggleCompact
        );
        // Without Ctrl, 'o' is just a literal character.
        assert_eq!(
            chat_key(&Key::Character("o".into()), true, false, false),
            ChatInput::Char('o')
        );
    }
}

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
    /// Tab — complete the leading @agent / /construct token, or take the
    /// ghost suggestion when there is no token to complete.
    Complete,
    /// Right arrow — take the ghost suggestion. The input bar's second accept
    /// key, so the two composers agree on how a suggestion is taken.
    Accept,
    /// Arrow keys — navigate a popup when one is open, otherwise walk the
    /// composer's prompt history (see `chathistory`).
    Up,
    Down,
    /// Ctrl+R — open the reverse history-search popup, or step to the next
    /// older match while it is open (see `chathistsearch`).
    HistSearch,
    /// Ctrl+F — open the transcript find popup, or step to the next older
    /// match while it is open (see `chatfind`; Cmd+F opens it app-side).
    FindNext,
    Ignore,
}

/// An action a chat pane asks the app to take after a key press.
pub(crate) enum ChatAction {
    /// Close this pane (Escape).
    Close,
    /// A `/theme` switch in the composer changed the live theme; the app
    /// must persist it (the pane can't reach the config) and refresh the
    /// theme-following accent — same pairing as every theme-change path.
    PersistTheme,
    /// `/font <arg>` typed in the composer: run it through the app's
    /// input-bar font path (size set / rotation toggle needs the renderer,
    /// which the pane can't reach).
    Font(String),
    /// The find popup moved its match target: scroll the transcript to it.
    /// App-side because the jump needs the pane's grid geometry, which the
    /// key handler doesn't have (see `chatfind::jump`).
    FindJump,
}

/// Classify a key press for a chat pane. Only presses act; Escape closes.
/// `shift` turns Enter into a newline instead of a send; `ctrl` decodes the
/// Ctrl+R history-search chord (winit keeps the logical key as the plain
/// character under Control, same as the `keys.rs` intercepts). (Ctrl+O — the
/// compact-transcript toggle — is no longer decoded here: it's a global
/// intercept in `keys.rs`, same reach as Ctrl+Shift+M.)
pub(crate) fn chat_key(logical: &Key, pressed: bool, shift: bool, ctrl: bool) -> ChatInput {
    if !pressed {
        return ChatInput::Ignore;
    }
    match logical {
        Key::Character(s) if ctrl && s.eq_ignore_ascii_case("r") => ChatInput::HistSearch,
        Key::Character(s) if ctrl && s.eq_ignore_ascii_case("f") => ChatInput::FindNext,
        Key::Named(NamedKey::Escape) => ChatInput::Close,
        Key::Named(NamedKey::Tab) => ChatInput::Complete,
        Key::Named(NamedKey::ArrowRight) => ChatInput::Accept,
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
#[path = "chatkeys_tests.rs"]
mod tests;

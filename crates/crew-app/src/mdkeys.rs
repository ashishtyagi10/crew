//! Key reduction for the markdown viewer: Tab side-switch, Up/Down/PageUp/
//! PageDown scrolling of the active half, `r` reload, and Esc to close —
//! split into a pure, testable seam the same way `chatkeys`/`farpane::keys`
//! are (winit's `KeyEvent` is `#[non_exhaustive]` and can't be built in
//! tests, so `MdPane::on_key` classifies it to `MdInput` first; everything
//! below that point is plain data this file's tests drive directly).
use winit::keyboard::{Key, NamedKey};

use crate::mdpane::MdPane;

/// A Page Up/Down jump scrolls this many lines. `MdPane::on_key` only has the
/// model at this layer, not the pane's row count (`keys.rs` doesn't thread
/// the grid through key routing), so this mirrors `farpane::keys::PAGE`'s
/// fixed page size rather than a real page height.
const PAGE: i32 = 10;

/// What a key press means to the markdown viewer, decoded from a winit
/// `KeyEvent`'s logical key + pressed state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MdInput {
    Close,
    Tab,
    Up,
    Down,
    PageUp,
    PageDown,
    Reload,
    Ignore,
}

/// An action the viewer asks the host app to take after a key press.
pub(crate) enum MdAction {
    /// Close this pane (Escape) — mirrors `ChatAction::Close`/`FarAction::Close`.
    Close,
    /// Show a transient status message (a failed `r` reload).
    Status(String),
}

/// Classify a key press. Only presses act; releases are ignored.
pub(crate) fn md_key(logical: &Key, pressed: bool) -> MdInput {
    if !pressed {
        return MdInput::Ignore;
    }
    match logical {
        Key::Named(NamedKey::Escape) => MdInput::Close,
        Key::Named(NamedKey::Tab) => MdInput::Tab,
        Key::Named(NamedKey::ArrowUp) => MdInput::Up,
        Key::Named(NamedKey::ArrowDown) => MdInput::Down,
        Key::Named(NamedKey::PageUp) => MdInput::PageUp,
        Key::Named(NamedKey::PageDown) => MdInput::PageDown,
        Key::Character(s) if s.eq_ignore_ascii_case("r") => MdInput::Reload,
        _ => MdInput::Ignore,
    }
}

/// Apply a classified key to the pane, returning an action for the host app
/// when one is needed (close, or a reload-failure status).
pub(crate) fn reduce(p: &mut MdPane, input: MdInput) -> Option<MdAction> {
    let active = p.active;
    match input {
        MdInput::Close => return Some(MdAction::Close),
        MdInput::Tab => p.active = active.other(),
        MdInput::Up => p.scroll(active, 1),
        MdInput::Down => p.scroll(active, -1),
        MdInput::PageUp => p.scroll(active, PAGE),
        MdInput::PageDown => p.scroll(active, -PAGE),
        MdInput::Reload => {
            if let Err(msg) = p.reload() {
                return Some(MdAction::Status(msg));
            }
        }
        MdInput::Ignore => {}
    }
    None
}

#[cfg(test)]
#[path = "mdkeys_tests.rs"]
mod tests;

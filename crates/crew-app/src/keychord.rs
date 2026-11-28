//! Two key *predicates* pulled out of [`crate::keys`]'s dispatch: the arrow
//! that names a grid direction, and the chord that toggles a chat pane's
//! compact view. Both are pure functions over a `winit` key so the mapping is
//! testable without constructing a `KeyEvent`.
use winit::keyboard::{Key, KeyCode, ModifiersState, NamedKey};

use crate::winkeys::WinChord;

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
/// Ctrl+Shift+M intercept ([`special`]): same reach (fires before the input-bar
/// early-return), but with no Shift requirement.
pub(crate) fn is_compact_chord(key: &Key, mods: winit::keyboard::ModifiersState) -> bool {
    mods.control_key()
        && !mods.shift_key()
        && matches!(key, Key::Character(s) if s.eq_ignore_ascii_case("o"))
}

/// The Cmd chord a key press makes, if any: the character a held Cmd (Super)
/// gives on a Mac — empty for a key that names none, which a held Cmd still
/// swallows — or, off a Mac, what [`crate::winkeys::chord`] makes of the key
/// (Alt+D is `"d"`). See [`cmd_string`].
pub(crate) fn cmd_chord(event: &winit::event::KeyEvent, mods: ModifiersState) -> Option<String> {
    use winit::keyboard::PhysicalKey;
    use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;
    if !event.state.is_pressed() {
        return None;
    }
    let code = match event.physical_key {
        PhysicalKey::Code(c) => Some(c),
        PhysicalKey::Unidentified(_) => None,
    };
    let bare = event.key_without_modifiers();
    cmd_string(
        &event.logical_key,
        &bare,
        code,
        mods,
        cfg!(target_os = "macos"),
    )
}

/// [`cmd_chord`] over the parts of a key press, with `mac` a parameter so
/// both platforms' keys are tested on one. `unmodified`: the key as if no
/// modifier were held; `code`: the physical key.
pub(crate) fn cmd_string(
    logical: &Key,
    unmodified: &Key,
    code: Option<KeyCode>,
    mods: ModifiersState,
    mac: bool,
) -> Option<String> {
    if mac {
        return mods.super_key().then(|| match logical {
            Key::Character(s) => s.to_string(),
            _ => String::new(),
        });
    }
    match crate::winkeys::chord(unmodified, code, mods) {
        Some(WinChord::Cmd(s)) => Some(s.to_string()),
        _ => None,
    }
}

/// The chords `keys.rs` handles before the Cmd chords: quit, and the theme,
/// gradient, focus-mode and markdown-source walks. On a Mac that is Cmd+Q
/// (or Ctrl+Q) and Ctrl+Shift+L/G/F/M; off one, Alt+Q/L/B/H/Y
/// ([`crate::winkeys`]), and plain Ctrl stays with whatever is typed into.
pub(crate) fn special(
    logical: &Key,
    unmodified: &Key,
    code: Option<KeyCode>,
    mods: ModifiersState,
    mac: bool,
) -> Option<WinChord> {
    if !mac {
        return crate::winkeys::chord(unmodified, code, mods).filter(|c| {
            matches!(
                c,
                WinChord::Quit
                    | WinChord::Theme
                    | WinChord::Gradient
                    | WinChord::Focus
                    | WinChord::MdSource
            )
        });
    }
    let Key::Character(s) = logical else {
        return None;
    };
    if (mods.super_key() || mods.control_key()) && s.as_str() == "q" {
        return Some(WinChord::Quit);
    }
    if !(mods.control_key() && mods.shift_key()) {
        return None;
    }
    match s.to_ascii_lowercase().as_str() {
        "l" => Some(WinChord::Theme),
        "g" => Some(WinChord::Gradient),
        "f" => Some(WinChord::Focus),
        "m" => Some(WinChord::MdSource),
        _ => None,
    }
}

/// A Ctrl+letter (not AltGr's Ctrl+Alt): no text to type. Far's command line
/// and a settings field read keys without Ctrl, so one left to them typed a
/// stray letter. Off a Mac plain Ctrl is no chord at all, so this is what
/// keeps those panes from typing for it.
pub(crate) fn ctrl_letter(key: &Key, mods: ModifiersState) -> bool {
    mods.control_key()
        && !mods.alt_key()
        && matches!(key, Key::Character(s) if s.chars().all(|c| c.is_ascii_alphabetic()))
}

#[cfg(test)]
#[path = "keychord_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "keychordoff_tests.rs"]
mod off_tests;

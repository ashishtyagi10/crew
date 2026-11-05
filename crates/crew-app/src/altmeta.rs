//! Alt as Meta: Alt+<key> sends `ESC` then the key, as xterm, Ghostty and
//! Windows Terminal send it.
//!
//! Alt+letter used to send the bare letter. Off a Mac Alt composes nothing,
//! so readline's M-b / M-f / M-d / M-. (last argument), emacs's M-x and
//! htop's or weechat's Alt-combos all arrived as plain typing: Alt+. put a
//! dot on the line. Only Alt+←/→ and Alt+Backspace had Meta (`keyenc`).
//!
//! On a Mac, Option IS a typing key — Option+e is the acute dead key, and
//! Option+2 is ™ — so Meta there is a setting (`option_as_meta`, off by
//! default), as in Terminal.app and iTerm2. With it on, the key is the one
//! printed on the keycap (`key_without_modifiers`), since what Option
//! composed (`∫` for b) is not the letter the program binds.
use crate::keyenc::Mods;
use winit::event::KeyEvent;
use winit::keyboard::Key;

/// The bytes Alt+<key> sends when Alt is Meta, or `None` to type the key as
/// it is. `typed`: the text the key gave with its modifiers; `bare`: the
/// keycap's own, unshifted. `mac`: Option is the Alt key; `option_as_meta`:
/// the user made it Meta there.
///
/// Ctrl+Alt is left alone: off a Mac it is how Windows reports AltGr, which
/// types `@`, `{` and `€` on most European layouts. On a Mac Shift with a
/// non-letter is too: the unshifted keycap is not the symbol Shift asks for
/// (Option+Shift+. means `>`, not `.`), so the composed key types instead.
pub(crate) fn meta(
    typed: &str,
    bare: &str,
    m: Mods,
    mac: bool,
    option_as_meta: bool,
) -> Option<Vec<u8>> {
    if !m.alt || m.ctrl || (mac && !option_as_meta) {
        return None;
    }
    let text = if !mac {
        typed.to_string()
    } else if !m.shift {
        bare.to_string()
    } else if bare.chars().all(|c| c.is_ascii_alphabetic()) {
        bare.to_ascii_uppercase()
    } else {
        return None;
    };
    if text.is_empty() || text.chars().any(char::is_control) {
        return None;
    }
    let mut out = vec![0x1b];
    out.extend_from_slice(text.as_bytes());
    Some(out)
}

/// [`meta`] for a winit key press.
pub(crate) fn meta_key(event: &KeyEvent, m: Mods, option_as_meta: bool) -> Option<Vec<u8>> {
    use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;
    if !event.state.is_pressed() {
        return None;
    }
    let (Key::Character(typed), Key::Character(bare)) =
        (&event.logical_key, &event.key_without_modifiers())
    else {
        return None;
    };
    meta(typed, bare, m, cfg!(target_os = "macos"), option_as_meta)
}

#[cfg(test)]
#[path = "altmeta_tests.rs"]
mod tests;

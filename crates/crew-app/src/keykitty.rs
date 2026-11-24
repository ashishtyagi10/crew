//! The kitty keyboard protocol's "disambiguate escape codes" flag (0b1): the
//! keys a program that asked for it gets as `CSI … u`.
//!
//! Without it a program can't tell Ctrl+I from Tab, Ctrl+M from Enter, a
//! lone Esc from the start of an Alt chord, Shift+Enter from Enter, or
//! Ctrl+Shift+A from Ctrl+A. nvim, helix, fish 4 and Claude Code push the
//! flag (`CSI > 1 u`); crew-term keeps the stack and says when it is on
//! (`InputModes::kitty_keys`). Spec: sw.kovidgoyal.net/kitty/keyboard-protocol.
//!
//! What changes under the flag, and only that ([`named`], [`text`]):
//! - Esc is `CSI 27 u` (with its modifiers, `CSI 27 ; m u`).
//! - Ctrl and Alt with a text key are `CSI code ; m u`, the code the
//!   unshifted key's (Ctrl+Shift+A is `97;6`, never `65`); Ctrl+Space `32;5`.
//! - Enter, Tab and Backspace stay `CR`, `HT`, `DEL` alone; held with a
//!   modifier they are `CSI 13/9/127 ; m u` (Shift+Enter `13;2u`).
//! - Modified arrows, Home, End, PgUp/PgDn, Insert, Delete and F1–F12 take
//!   xterm's `CSI 1 ; m X` / `CSI n ; m ~` forms, F3 as `13 ; m ~` (`CSI R`
//!   is a cursor report). Unmodified, they stay the legacy bytes (`keyenc`).
//! - Text typed plain or with Shift stays UTF-8.
//!
//! `m` is 1 + shift + 2·alt + 4·ctrl. A Mac's Option is a typing key unless
//! `option_as_meta` makes it Meta, and Ctrl+Alt off a Mac is how Windows
//! reports AltGr: those type their text, as in `altmeta`. Crew's own chords
//! (Cmd+…, Ctrl+Tab, Ctrl+Q) are taken before a key gets here.
use crate::keyenc::Mods;
use winit::event::KeyEvent;
use winit::keyboard::{Key, NamedKey};

/// xterm's (and kitty's) modifier parameter.
fn param(m: Mods) -> u8 {
    1 + u8::from(m.shift) + 2 * u8::from(m.alt) + 4 * u8::from(m.ctrl)
}

/// `CSI code u`, or `CSI code ; m u` when a modifier is held.
fn csi_u(code: u32, p: u8) -> String {
    match p {
        1 => format!("\x1b[{code}u"),
        _ => format!("\x1b[{code};{p}u"),
    }
}

/// The bytes for named key `n` held with `m`, or `None` where the legacy
/// encoding stands.
pub(crate) fn named(n: NamedKey, m: Mods) -> Option<Vec<u8>> {
    use NamedKey::*;
    let p = param(m);
    let seq = match n {
        Escape => csi_u(27, p),
        Enter if p > 1 => csi_u(13, p),
        Tab if p > 1 => csi_u(9, p),
        Backspace if p > 1 => csi_u(127, p),
        Space if m.ctrl || m.alt => csi_u(32, p),
        _ if p == 1 => return None,
        ArrowUp => format!("\x1b[1;{p}A"),
        ArrowDown => format!("\x1b[1;{p}B"),
        ArrowRight => format!("\x1b[1;{p}C"),
        ArrowLeft => format!("\x1b[1;{p}D"),
        Home => format!("\x1b[1;{p}H"),
        End => format!("\x1b[1;{p}F"),
        F1 => format!("\x1b[1;{p}P"),
        F2 => format!("\x1b[1;{p}Q"),
        F4 => format!("\x1b[1;{p}S"),
        _ => format!("\x1b[{};{p}~", tilde(n)?),
    };
    Some(seq.into_bytes())
}

/// The `CSI n ~` number of the keys that have one.
fn tilde(n: NamedKey) -> Option<u8> {
    use NamedKey::*;
    Some(match n {
        Insert => 2,
        Delete => 3,
        PageUp => 5,
        PageDown => 6,
        F3 => 13,
        F5 => 15,
        F6 => 17,
        F7 => 18,
        F8 => 19,
        F9 => 20,
        F10 => 21,
        F11 => 23,
        F12 => 24,
        _ => return None,
    })
}

/// The bytes for a text key: `typed` what it typed with its modifiers,
/// `bare` the keycap's own character. `None` types it as before.
pub(crate) fn text(
    typed: &str,
    bare: &str,
    m: Mods,
    mac: bool,
    option_as_meta: bool,
) -> Option<Vec<u8>> {
    let altgr = !mac && m.ctrl && m.alt && typed != bare && printable(typed);
    if altgr {
        return None;
    }
    let m = Mods {
        alt: m.alt && (!mac || option_as_meta),
        ..m
    };
    if !m.ctrl && !m.alt {
        return None;
    }
    let mut cs = bare.chars();
    let (Some(c), None) = (cs.next(), cs.next()) else {
        return None;
    };
    if c.is_control() {
        return None;
    }
    let code = c.to_lowercase().next().unwrap_or(c);
    Some(csi_u(u32::from(code), param(m)).into_bytes())
}

fn printable(s: &str) -> bool {
    !s.is_empty() && !s.chars().any(char::is_control)
}

/// [`encode`] for a winit key press.
pub(crate) fn key_event(
    event: &KeyEvent,
    m: Mods,
    kitty: bool,
    option_as_meta: bool,
) -> Option<Vec<u8>> {
    use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;
    if !kitty || !event.state.is_pressed() {
        return None;
    }
    let bare = event.key_without_modifiers();
    let mac = cfg!(target_os = "macos");
    encode(
        &event.logical_key,
        &bare,
        event.text.as_deref(),
        m,
        kitty,
        mac,
        option_as_meta,
    )
}

/// [`named`] / [`text`] for a key: `logical` as pressed, `bare` without its
/// modifiers, `typed` the text it produced. Only when the focused program
/// turned the flag on (`kitty`); `None` leaves the key to the legacy path.
pub(crate) fn encode(
    logical: &Key,
    bare: &Key,
    typed: Option<&str>,
    m: Mods,
    kitty: bool,
    mac: bool,
    option_as_meta: bool,
) -> Option<Vec<u8>> {
    if !kitty {
        return None;
    }
    if let Key::Named(n) = logical {
        return named(*n, m);
    }
    let (Key::Character(own), Key::Character(bare)) = (logical, bare) else {
        return None;
    };
    let typed = typed.filter(|t| printable(t)).unwrap_or(own);
    text(typed, bare, m, mac, option_as_meta)
}

#[cfg(test)]
#[path = "keykitty_tests.rs"]
mod tests;

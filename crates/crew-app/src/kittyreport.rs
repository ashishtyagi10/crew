//! The kitty keyboard protocol's flags 2, 4, 8 and 16 on top of flag 1
//! (`keykitty`): what Neovim, Helix, fish 4, Zellij and Kakoune ask for.
//!
//! alacritty_terminal already keeps the stack, answers `CSI ? u` and sets the
//! mode bits (`InputModes::kitty_flags`); this is the encoder half. A key is
//! `CSI code[:shifted[:base]] ; mods[:event] ; text u`:
//! - 2 event types: a press is `:1` (left off), a repeat `:2`, a release `:3`
//!   (with no modifier held, `;1:3`). Only keys that are escape codes report
//!   them: arrows and F-keys always, Esc and the chords under flag 1, and
//!   under 8 every key, Enter/Tab/Backspace and the modifier keys too.
//! - 4 alternate keys: the shifted character (only with Shift held) and the
//!   key's PC-101 base-layout character when the layout types another.
//! - 8 all keys as escape codes: plain and Shift-ed text, Enter, Tab,
//!   Backspace, the keypad (`57399…`) and bare modifier keys.
//! - 16 associated text: the typed codepoints ride last, never on a release
//!   or a chord.
//!
//! Text the layout composed (Option+letter on a Mac, AltGr, a dead key's
//! accent) is not a key's own text: it goes as plain text unless 8 and 16
//! both ask for text as codepoints, and an IME commit never comes through
//! here at all (`composeinput` writes it raw). Caps/Num Lock bits are not
//! reported. `keykitty` keeps the flag-1-only forms; this is for the rest.
use winit::keyboard::{Key, KeyCode, NamedKey};

use crate::keyenc::Mods;
use crate::kittytable::{
    base_layout, codepoints, csi_u, enter_code, functional, keypad, modifier, param,
};

pub(crate) const EVENTS: u8 = 2;
pub(crate) const ALTERNATES: u8 = 4;
pub(crate) const ALL_KEYS: u8 = 8;
pub(crate) const TEXT: u8 = 16;

/// Press, auto-repeat or release.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Press,
    Repeat,
    Release,
}

/// One key event, as much of a winit `KeyEvent` as the encoder reads.
pub(crate) struct Ev<'a> {
    pub logical: &'a Key,
    pub bare: &'a Key,
    pub text: Option<&'a str>,
    pub code: Option<KeyCode>,
    pub numpad: bool,
    pub kind: Kind,
}

/// The bytes for `e` with `m` held under `flags`, or `None` where the legacy
/// encoding (or nothing: a release of a typed letter) stands.
pub(crate) fn encode(flags: u8, e: &Ev, m: Mods, mac: bool, meta: bool) -> Option<Vec<u8>> {
    let events = flags & EVENTS != 0;
    let ev: u8 = match e.kind {
        Kind::Release if !events => return None,
        Kind::Release => 3,
        Kind::Repeat if events => 2,
        _ => 1,
    };
    let all = flags & ALL_KEYS != 0;
    let esc = all || flags & 1 != 0;
    let bits = param(m) - 1;
    let p = 1 + bits;
    if let Some((code, own)) = e.code.and_then(modifier) {
        // The key's own press already counts in the modifiers it reports.
        let held = if ev == 3 { bits & !own } else { bits | own };
        return all.then(|| csi_u(code.to_string(), 1 + held, ev, None).into_bytes());
    }
    if let Some(code) = e.code.and_then(keypad).filter(|_| e.numpad) {
        let text = (flags & TEXT != 0 && ev != 3)
            .then(|| e.text.map(codepoints))
            .flatten();
        return all.then(|| csi_u(code.to_string(), p, ev, text).into_bytes());
    }
    let seq = match e.logical {
        Key::Named(NamedKey::Escape) => esc.then(|| csi_u("27".into(), p, ev, None))?,
        Key::Named(n @ (NamedKey::Enter | NamedKey::Tab | NamedKey::Backspace)) => {
            (all || (esc && p > 1)).then(|| csi_u(enter_code(*n).to_string(), p, ev, None))?
        }
        Key::Named(NamedKey::Space) => text_key(flags, e, " ", " ", m, ev, mac, meta)?,
        Key::Named(n) => functional(*n, p, ev)?,
        Key::Character(own) => {
            let Key::Character(bare) = e.bare else {
                return None;
            };
            text_key(flags, e, own, bare, m, ev, mac, meta)?
        }
        _ => return None,
    };
    Some(seq.into_bytes())
}

#[allow(clippy::too_many_arguments)]
fn text_key(
    flags: u8,
    e: &Ev,
    own: &str,
    bare: &str,
    m: Mods,
    ev: u8,
    mac: bool,
    meta: bool,
) -> Option<String> {
    let typed = e
        .text
        .filter(|t| !t.is_empty() && !t.chars().any(char::is_control));
    let typed = typed.unwrap_or(own);
    let all = flags & ALL_KEYS != 0;
    // Option types a character on a Mac unless it is Meta; Ctrl+Alt off a Mac
    // is AltGr. Both leave `typed` as composed text, not the key's own.
    let option = mac && m.alt && !meta;
    let altgr = !mac && m.ctrl && m.alt && typed != bare;
    let m = Mods {
        alt: m.alt && !option && !altgr,
        ctrl: m.ctrl && !altgr,
        ..m
    };
    let composed = option || altgr;
    if composed && !(all && flags & TEXT != 0) {
        return None;
    }
    if !all && !(flags & 1 != 0 && (m.ctrl || m.alt)) {
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
    let mut field = u32::from(code).to_string();
    if flags & ALTERNATES != 0 {
        let shifted = own
            .chars()
            .next()
            .filter(|s| m.shift && own.chars().count() == 1 && *s != c);
        let base = e.code.and_then(base_layout).filter(|b| *b != code);
        match (shifted, base) {
            (None, None) => {}
            (s, b) => {
                field += &format!(
                    ":{}",
                    s.map(|s| u32::from(s).to_string()).unwrap_or_default()
                );
                if let Some(b) = b {
                    field += &format!(":{}", u32::from(b));
                }
            }
        }
    }
    let text = (flags & TEXT != 0 && ev != 3 && !m.ctrl && !m.alt).then(|| codepoints(typed));
    Some(csi_u(field, param(m), ev, text))
}

#[cfg(test)]
#[path = "kittyreport_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "kittyreport2_tests.rs"]
mod tests_more;

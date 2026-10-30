//! The bytes a named key sends a terminal program when it needs more than the
//! plain table in `session`: function keys, Option-word motion, and the
//! cursor keys' application mode.
//!
//! F1–F12 sent nothing at all — htop's F10, mc's whole bottom row and vim's
//! mapped F-keys were dead in a crew pane. Option+← / → and Option+Backspace
//! sent a bare arrow and a bare DEL, so the word jump and word delete every
//! macOS terminal gives a shell moved one character. And a program that
//! switched the cursor keys to application mode (DECCKM) still got the
//! normal-mode arrows.
//!
//! The words go as `ESC b` / `ESC f` / `ESC DEL` — readline's and zsh's own
//! word bindings, as Terminal.app and Ghostty send them on a Mac. Shift- and
//! Ctrl-arrows stay plain on purpose: their xterm forms (`ESC [1;2D`) are
//! unbound in a bare zsh or bash, which swallowed them AND the next key typed
//! (measured 2026-10-30).
use winit::keyboard::NamedKey;

/// The modifiers held with a key, as a terminal encodes them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Mods {
    pub shift: bool,
    pub alt: bool,
    pub ctrl: bool,
}

impl Mods {
    /// xterm's modifier parameter: 1 plus shift 1, alt 2, ctrl 4.
    fn param(self) -> u8 {
        1 + u8::from(self.shift) + 2 * u8::from(self.alt) + 4 * u8::from(self.ctrl)
    }

    fn none(self) -> bool {
        self == Self::default()
    }

    fn alt_only(self) -> bool {
        self == Self {
            alt: true,
            ..Self::default()
        }
    }
}

/// The bytes for `n` held with `m`, or `None` for a key the plain table in
/// `session` answers. `app_cursor`: the program set DECCKM.
pub(crate) fn named(n: NamedKey, m: Mods, app_cursor: bool) -> Option<Vec<u8>> {
    if m.alt_only() {
        match n {
            NamedKey::ArrowLeft => return Some(b"\x1bb".to_vec()),
            NamedKey::ArrowRight => return Some(b"\x1bf".to_vec()),
            NamedKey::Backspace => return Some(b"\x1b\x7f".to_vec()),
            _ => {}
        }
    }
    if let Some(f) = function(n) {
        // F1–F4 are SS3 (`ESC O P`), CSI with a modifier; F5 on are `CSI n ~`.
        let ss3 = (f <= 4).then(|| b"PQRS"[usize::from(f - 1)] as char);
        let seq = match (ss3, m.none()) {
            (Some(c), true) => format!("\x1bO{c}"),
            (Some(c), false) => format!("\x1b[1;{}{c}", m.param()),
            (None, true) => format!("\x1b[{}~", tilde(f)),
            (None, false) => format!("\x1b[{};{}~", tilde(f), m.param()),
        };
        return Some(seq.into_bytes());
    }
    let letter = match n {
        NamedKey::ArrowUp => b'A',
        NamedKey::ArrowDown => b'B',
        NamedKey::ArrowRight => b'C',
        NamedKey::ArrowLeft => b'D',
        NamedKey::Home => b'H',
        NamedKey::End => b'F',
        _ => return None,
    };
    (app_cursor && m.none()).then(|| vec![0x1b, b'O', letter])
}

/// F1 → 1 … F12 → 12.
fn function(n: NamedKey) -> Option<u8> {
    use NamedKey::*;
    let keys = [F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12];
    keys.iter().position(|k| *k == n).map(|i| i as u8 + 1)
}

/// The `CSI n ~` number of F5–F12 (xterm skips 16 and 22).
fn tilde(f: u8) -> u8 {
    [15, 17, 18, 19, 20, 21, 23, 24][usize::from(f - 5)]
}

#[cfg(test)]
#[path = "keyenc_tests.rs"]
mod tests;

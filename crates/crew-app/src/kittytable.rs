//! The kitty keyboard protocol's key tables and sequence forms beyond what
//! flag 1 needs: the PC-101 base-layout character of a physical key (flag
//! 4), the keypad's and the modifier keys' private codes (flag 8), and the
//! `CSI … u` / functional-key shapes `kittyreport` fills. Spec: "Functional key
//! definitions" at sw.kovidgoyal.net/kitty/keyboard-protocol.
use crate::keyenc::Mods;
use winit::keyboard::{KeyCode, NamedKey};

/// The character a physical key types on a US PC-101 layout, unshifted.
/// Flag 4 reports it so a program's `ctrl+c` binding still fires on a
/// Cyrillic or Greek layout, where the key types `с` or `ψ` instead.
pub(crate) fn base_layout(code: KeyCode) -> Option<char> {
    use KeyCode::*;
    let letters = [
        KeyA, KeyB, KeyC, KeyD, KeyE, KeyF, KeyG, KeyH, KeyI, KeyJ, KeyK, KeyL, KeyM, KeyN, KeyO,
        KeyP, KeyQ, KeyR, KeyS, KeyT, KeyU, KeyV, KeyW, KeyX, KeyY, KeyZ,
    ];
    if let Some(i) = letters.iter().position(|k| *k == code) {
        return Some((b'a' + i as u8) as char);
    }
    let digits = [
        Digit0, Digit1, Digit2, Digit3, Digit4, Digit5, Digit6, Digit7, Digit8, Digit9,
    ];
    if let Some(i) = digits.iter().position(|k| *k == code) {
        return Some((b'0' + i as u8) as char);
    }
    Some(match code {
        Minus => '-',
        Equal => '=',
        BracketLeft => '[',
        BracketRight => ']',
        Backslash => '\\',
        Semicolon => ';',
        Quote => '\'',
        Backquote => '`',
        Comma => ',',
        Period => '.',
        Slash => '/',
        _ => return None,
    })
}

/// A keypad key's private-use code: KP_0..KP_9 are 57399..57408, then
/// decimal, divide, multiply, subtract, add, enter, equal.
pub(crate) fn keypad(code: KeyCode) -> Option<u32> {
    use KeyCode::*;
    let digits = [
        Numpad0, Numpad1, Numpad2, Numpad3, Numpad4, Numpad5, Numpad6, Numpad7, Numpad8, Numpad9,
    ];
    if let Some(i) = digits.iter().position(|k| *k == code) {
        return Some(57399 + i as u32);
    }
    Some(match code {
        NumpadDecimal => 57409,
        NumpadDivide => 57410,
        NumpadMultiply => 57411,
        NumpadSubtract => 57412,
        NumpadAdd => 57413,
        NumpadEnter => 57414,
        NumpadEqual => 57415,
        _ => return None,
    })
}

/// A modifier key's code and the modifier bit it sets (xterm's 1 shift, 2
/// alt, 4 ctrl, 8 super): the key's own press already counts in the
/// modifiers it reports.
pub(crate) fn modifier(code: KeyCode) -> Option<(u32, u8)> {
    use KeyCode::*;
    Some(match code {
        ShiftLeft => (57441, 1),
        ControlLeft => (57442, 4),
        AltLeft => (57443, 2),
        SuperLeft => (57444, 8),
        ShiftRight => (57447, 1),
        ControlRight => (57448, 4),
        AltRight => (57449, 2),
        SuperRight => (57450, 8),
        _ => return None,
    })
}

pub(crate) fn param(m: Mods) -> u8 {
    1 + u8::from(m.shift) + 2 * u8::from(m.alt) + 4 * u8::from(m.ctrl)
}

pub(crate) fn enter_code(n: NamedKey) -> u32 {
    match n {
        NamedKey::Enter => 13,
        NamedKey::Tab => 9,
        _ => 127,
    }
}

/// The text `s` as colon-joined codepoints.
pub(crate) fn codepoints(s: &str) -> String {
    let cs: Vec<String> = s.chars().map(|c| u32::from(c).to_string()).collect();
    cs.join(":")
}

/// `CSI code ; mods[:ev] [; text] u`, leaving off what a plain press omits.
pub(crate) fn csi_u(code: String, p: u8, ev: u8, text: Option<String>) -> String {
    let mods = match (p, ev) {
        (_, 2..) => format!("{p}:{ev}"),
        (1, _) => String::new(),
        _ => p.to_string(),
    };
    match (mods.is_empty(), text) {
        (true, None) => format!("\x1b[{code}u"),
        (_, None) => format!("\x1b[{code};{mods}u"),
        (_, Some(t)) => format!("\x1b[{code};{mods};{t}u"),
    }
}

/// Arrows, Home, End, Insert, Delete, PgUp/PgDn and F1–F12: always escape
/// codes, so an event type or a modifier changes them and nothing else does.
pub(crate) fn functional(n: NamedKey, p: u8, ev: u8) -> Option<String> {
    use NamedKey::*;
    if p == 1 && ev == 1 {
        return None;
    }
    let tail = if ev > 1 {
        format!("{p}:{ev}")
    } else {
        p.to_string()
    };
    let letter = match n {
        ArrowUp => 'A',
        ArrowDown => 'B',
        ArrowRight => 'C',
        ArrowLeft => 'D',
        Home => 'H',
        End => 'F',
        F1 => 'P',
        F2 => 'Q',
        F4 => 'S',
        _ => return Some(format!("\x1b[{};{tail}~", crate::keykitty::tilde(n)?)),
    };
    Some(format!("\x1b[1;{tail}{letter}"))
}

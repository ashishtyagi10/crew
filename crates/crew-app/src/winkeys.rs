//! crew's keys off a Mac: **Alt is crew's key**, and every shortcut is one
//! modifier plus one key (docs/plan/window-keys.md).
//!
//! The user, 2026-11-25: "I don't like [three] key combination, re-assign keys
//! so that everything can be driven by two key combination in windows." Plain
//! Ctrl can't be crew's there — a shell needs Ctrl+C, Ctrl+D, Ctrl+R — so Alt,
//! which sits where a Mac's Cmd does, takes Cmd's place: Cmd+D is Alt+D. The
//! Mac's shifted and Opt chords get a free letter or a named key of their
//! own, and Ctrl stays with whatever is being typed into.
//!
//! This is the table and nothing else: a pure function from a key press to
//! the chord it makes, so every row is testable without a window. What a
//! [`WinChord::Cmd`] does is `handle_super_chord`'s, the same as on a Mac.
use winit::keyboard::{Key, KeyCode, ModifiersState, NamedKey};

/// What a key press off a Mac asks crew for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WinChord {
    /// The chord a Mac's Cmd makes, as `handle_super_chord` reads it: `"d"`
    /// for Cmd+D, `"T"` for Cmd+Shift+T.
    Cmd(&'static str),
    /// Ctrl+Shift+L on a Mac: cycle themes.
    Theme,
    /// Ctrl+Shift+G on a Mac: step the canvas gradient.
    Gradient,
    /// Ctrl+Shift+F on a Mac: focus mode.
    Focus,
    /// Ctrl+Shift+M on a Mac: a chat pane's markdown source.
    MdSource,
    /// The previous (-1) or next (+1) tab.
    Tab(i32),
    /// Tab N, counted from 1.
    TabN(usize),
    /// The previous (-1) or next (+1) screen (`panezoom`).
    Screen(i32),
    /// A terminal's previous (`true`) or next command (`cmdjump`).
    CmdJump(bool),
    /// Maximize the window — Cmd+M on a Mac, whose Alt+M is taken.
    Maximize,
    /// Quit — Cmd+Q, which `keys.rs` handles before the Cmd chords.
    Quit,
}

/// Alt+key → the Mac's Cmd chord. A letter that is not here (R) is left to
/// the terminal, which reads Alt as Meta.
const ALT: &[(&str, &str)] = &[
    ("a", "a"),
    ("c", "c"),
    ("d", "d"),
    ("e", "e"),
    ("f", "f"),
    ("g", "g"),
    ("i", "i"),
    ("j", "j"),
    ("k", "k"),
    ("n", "n"),
    ("o", "o"),
    ("p", "p"),
    ("s", "s"),
    ("t", "t"),
    ("v", "v"),
    ("w", "w"),
    ("z", "z"),
    (",", ","),
    (".", "."),
    (";", ";"), // fix a failed command with smith (`fixwithsmith`)
    ("/", "/"),
    ("[", "["),
    ("]", "]"),
    ("=", "="),
    ("-", "-"),
    ("0", "0"),
    // The Mac's three-key chords, given a letter of their own.
    ("u", "T"), // Cmd+Shift+T: reopen the closed pane
    ("x", "E"), // Cmd+Shift+E: the Explorer takes the keys
    ("m", "M"), // Cmd+Shift+M: send the pane to another tab
];

/// The Mac's spelling → this platform's, for every chord whose Windows key is
/// not simply Cmd written as Alt. The labels (`chordglyph`) read this, so a
/// label and its key come from one place and cannot drift.
pub(crate) const REMAPS: &[(&str, &str)] = &[
    ("Cmd+Shift+T", "Alt+U"),
    ("Cmd+Shift+E", "Alt+X"),
    ("Cmd+Shift+M", "Alt+M"),
    ("Cmd+M", "F11"),
    ("Cmd+{ / Cmd+}", "Ctrl+PgUp / Ctrl+PgDn"),
    ("Cmd+Opt+1 \u{2026} 9", "Ctrl+1 \u{2026} 9"),
    ("Cmd+Opt+\u{2191}\u{2193}", "Alt+PgUp / Alt+PgDn"),
    ("Cmd+Opt+\u{2190}\u{2192}", "Alt+Home / Alt+End"),
    ("Ctrl+Shift+L", "Alt+L"),
    ("Ctrl+Shift+G", "Alt+B"),
    ("Ctrl+Shift+F", "Alt+H"),
    ("Ctrl+Shift+M", "Alt+Y"),
];

/// The chord a key press makes off a Mac, or `None` when the key belongs to
/// the program in the pane or to the OS. `key` is the key as if no modifier
/// were held (`key_without_modifiers`), so Alt+Shift does not turn a letter
/// into its capital; `code` is the physical key, which names the digit row
/// on layouts whose digits need Shift (AZERTY's `&` is Digit1).
///
/// Never a chord: AltGr (which arrives as Ctrl+Alt), anything with the
/// Windows key (the OS keeps those), three-key presses, and Alt+F4, Alt+Tab
/// and Alt+Space, which close, switch and open the window menu.
pub(crate) fn chord(key: &Key, code: Option<KeyCode>, mods: ModifiersState) -> Option<WinChord> {
    let (ctrl, alt, shift) = (mods.control_key(), mods.alt_key(), mods.shift_key());
    if mods.super_key() || shift || (ctrl && alt) {
        return None;
    }
    if let Key::Named(n) = key {
        return named(*n, ctrl, alt);
    }
    if let Some(n) = digit(code) {
        return match (ctrl, alt) {
            (true, false) => Some(WinChord::TabN(n)),
            (false, true) => Some(WinChord::Cmd(DIGITS[n - 1])),
            _ => None,
        };
    }
    let Key::Character(s) = key else {
        return None;
    };
    if !alt {
        return None;
    }
    let k = s.to_lowercase();
    match k.as_str() {
        "l" => Some(WinChord::Theme),
        "b" => Some(WinChord::Gradient),
        "h" => Some(WinChord::Focus),
        "y" => Some(WinChord::MdSource),
        "q" => Some(WinChord::Quit),
        k => ALT
            .iter()
            .find(|(from, _)| *from == k)
            .map(|(_, to)| WinChord::Cmd(to)),
    }
}

/// A named key's chord: PgUp/PgDn step tabs with Ctrl and commands with Alt,
/// Home/End step screens with Alt, Alt+Enter is Cmd+Enter (an inbox reply
/// goes), and F11 alone maximizes.
fn named(n: NamedKey, ctrl: bool, alt: bool) -> Option<WinChord> {
    match (n, ctrl, alt) {
        (NamedKey::PageUp, true, false) => Some(WinChord::Tab(-1)),
        (NamedKey::PageDown, true, false) => Some(WinChord::Tab(1)),
        (NamedKey::PageUp, false, true) => Some(WinChord::CmdJump(true)),
        (NamedKey::PageDown, false, true) => Some(WinChord::CmdJump(false)),
        (NamedKey::Home, false, true) => Some(WinChord::Screen(-1)),
        (NamedKey::End, false, true) => Some(WinChord::Screen(1)),
        (NamedKey::Enter, false, true) => Some(WinChord::Cmd("")),
        (NamedKey::F11, false, false) => Some(WinChord::Maximize),
        _ => None,
    }
}

const DIGITS: [&str; 9] = ["1", "2", "3", "4", "5", "6", "7", "8", "9"];

/// 1 to 9 for the digit row's keys, wherever the layout puts its digits.
fn digit(code: Option<KeyCode>) -> Option<usize> {
    const ROW: [KeyCode; 9] = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
        KeyCode::Digit9,
    ];
    ROW.iter().position(|c| Some(*c) == code).map(|i| i + 1)
}

#[cfg(test)]
#[path = "winkeys_tests.rs"]
mod tests;

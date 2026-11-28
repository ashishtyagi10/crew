//! How a chord is written on this machine: `⇧⌘T` on macOS, the table's
//! `Cmd+Shift+T` in the tables — and off a Mac the key it is pressed with
//! there: Alt where the Mac has Cmd (`Cmd+I` → `Alt+I`), a key of its own for
//! the rest (`Cmd+Shift+T` → `Alt+U`, [`offmac`]).
//!
//! The welcome card and the settings form already wrote `⌘T` and `⌘S` on a
//! Mac, and `/keys` and the palette beside them still spelled `Cmd+T` — two
//! vocabularies for one key on one screen. The tables keep the portable
//! spelling (the README parity tests read them, and Windows and Linux draw
//! them as written); this is the one place a Mac's glyphs are put in, at
//! draw time.
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::winkeys::REMAPS;

/// Modifier words in the order a Mac writes their glyphs: ⌃⌥⇧⌘.
const MODS: [(&str, char); 5] = [
    ("Ctrl+", '\u{2303}'),
    ("Alt+", '\u{2325}'),
    ("Opt+", '\u{2325}'),
    ("Shift+", '\u{21e7}'),
    ("Cmd+", '\u{2318}'),
];

/// `k` as this platform writes it. Keys are a small fixed set, so each
/// rewritten one is kept for the life of the process and handed out as
/// `'static`, which is what the key tables are.
pub(crate) fn shown(k: &'static str) -> &'static str {
    if !rewrites(k) {
        return k;
    }
    static MEMO: OnceLock<Mutex<HashMap<&'static str, &'static str>>> = OnceLock::new();
    let mut memo = MEMO
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    memo.entry(k)
        .or_insert_with(|| Box::leak(native(k).into_boxed_str()))
}

/// [`shown`] for the document window and the editors, whose own keys take
/// Ctrl as Cmd off a Mac (`docwin::keys`, `viewedit`): their `Cmd+B` is
/// `Ctrl+B` there, not the main window's Alt+B, and redo (`Cmd+Shift+Z`) is
/// `Ctrl+Y`.
pub(crate) fn shown_ctrl(k: &'static str) -> &'static str {
    if cfg!(target_os = "macos") || !k.contains("Cmd+") {
        return shown(k);
    }
    static MEMO: OnceLock<Mutex<HashMap<&'static str, &'static str>>> = OnceLock::new();
    let mut memo = MEMO
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    memo.entry(k).or_insert_with(|| {
        let k = k.replace("Cmd+Shift+Z", "Ctrl+Y").replace("Cmd+", "Ctrl+");
        Box::leak(k.into_boxed_str())
    })
}

/// A sentence as this platform writes the chords in it — a status flash or
/// a toast (`no shell here — press ⌘T to open one`). Borrowed untouched
/// wherever there is no chord to rewrite.
pub(crate) fn prose(s: &str) -> std::borrow::Cow<'_, str> {
    match rewrites(s) {
        true => std::borrow::Cow::Owned(native(s)),
        false => std::borrow::Cow::Borrowed(s),
    }
}

/// [`prose`] for a sentence a Mac already shows as it is written (`press
/// Cmd+T`, which a Mac has always read as it stands): only off a Mac are its
/// chords rewritten, so a Mac's text does not change.
pub(crate) fn prose_off_mac(s: &str) -> std::borrow::Cow<'_, str> {
    match cfg!(target_os = "macos") {
        true => std::borrow::Cow::Borrowed(s),
        false => prose(s),
    }
}

/// Whether [`native`] would change `s`: any chord on a Mac, a Cmd one or a
/// remapped Ctrl+Shift one elsewhere.
fn rewrites(s: &str) -> bool {
    s.contains('+')
        && (cfg!(target_os = "macos")
            || s.contains("Cmd+")
            || s.contains("Ctrl+Shift+")
            || s.contains('\u{2318}'))
}

fn native(s: &str) -> String {
    match cfg!(target_os = "macos") {
        true => mac(s),
        false => offmac(s),
    }
}

/// Every chord in `s` as Windows and Linux press it (`winkeys`, plan step 6):
/// the chords with a key of their own come from [`REMAPS`] — the table the
/// key press reads too, so a label and its key cannot drift (`Cmd+Shift+T` is
/// `Alt+U`, `Cmd+M` is `F11`) — and every other Cmd chord is the same key
/// with Alt where the Mac has Cmd (`Cmd+T` is `Alt+T`). A click or the wheel
/// keeps Ctrl (`events::open_modifier`, `scroll`).
pub(crate) fn offmac(s: &str) -> String {
    let s = REMAPS
        .iter()
        .fold(s.to_string(), |s, (m, w)| swap(&s, m, w));
    let words: Vec<_> = s.split(' ').map(off_chord).collect();
    // `\u{2318}/Ctrl+S` names the two keys that save; off a Mac one of them.
    dedupe(&words.join(" ").replace("\u{2318}/", ""))
}

/// `Alt+S / Alt+S` once: a row that listed a Mac's `Cmd+S / Alt+S` pair
/// (the settings form's) names one key off a Mac.
fn dedupe(s: &str) -> String {
    let mut seen: Vec<&str> = Vec::new();
    for part in s.split(" / ") {
        if !(part.contains('+') && seen.contains(&part)) {
            seen.push(part);
        }
    }
    seen.join(" / ")
}

/// `s` with each whole-chord `from` written `to`: not when it is the tail of
/// a longer chord (`Cmd+M` inside `Cmd+Shift+M`) or the head of a longer key.
fn swap(s: &str, from: &str, to: &str) -> String {
    let (mut out, mut at) = (String::new(), 0);
    while let Some(i) = s[at..].find(from) {
        let (a, b) = (at + i, at + i + from.len());
        let edge = !s[..a].ends_with(|c: char| c.is_alphanumeric() || c == '+')
            && !s[b..].starts_with(char::is_alphanumeric);
        out.push_str(&s[at..a]);
        out.push_str(if edge { to } else { from });
        at = b;
    }
    out + &s[at..]
}

/// One space-free token of [`offmac`]; a trailing `.`/`,`/`)` rides along.
fn off_chord(tok: &str) -> String {
    let Some(rest) = tok.strip_prefix("Cmd+").filter(|r| !r.is_empty()) else {
        return tok.to_string();
    };
    match rest.trim_end_matches([',', ';', ')']) {
        "Click" | "click" | "wheel" => format!("Ctrl+{rest}"),
        _ => format!("Alt+{rest}"),
    }
}

/// Every chord in `s` in a Mac's glyphs: `Cmd+Shift+T` → `⇧⌘T`,
/// `Ctrl+Tab / Ctrl+Shift+Tab` → `⌃Tab / ⌃⇧Tab`. Words that are not chords
/// (`/`, `…`, `(in input)`) are left as they are.
pub(crate) fn mac(s: &str) -> String {
    s.split(' ').map(chord).collect::<Vec<_>>().join(" ")
}

/// One space-free token: its modifier prefixes as glyphs, in ⌃⌥⇧⌘ order,
/// then the key. A token with no modifier, or nothing after its modifiers
/// (`Cmd+` alone), is not a chord.
fn chord(tok: &str) -> String {
    let mut rest = tok;
    let mut held = [false; MODS.len()];
    while let Some(i) = MODS.iter().position(|(w, _)| rest.starts_with(w)) {
        held[i] = true;
        rest = &rest[MODS[i].0.len()..];
    }
    if rest.is_empty() || !held.contains(&true) {
        return tok.to_string();
    }
    let mut glyphs: Vec<char> = MODS
        .iter()
        .zip(held)
        .filter(|(_, h)| *h)
        .map(|((_, g), _)| *g)
        .collect();
    glyphs.dedup(); // `Alt+` and `Opt+` are one key
    glyphs.into_iter().collect::<String>() + rest
}

#[cfg(test)]
#[path = "chordglyph_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "chordsweep_tests.rs"]
mod sweep_tests;

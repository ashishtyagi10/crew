//! How a chord is written on this machine: `⇧⌘T` on macOS, `Cmd+Shift+T`
//! everywhere else.
//!
//! The welcome card and the settings form already wrote `⌘T` and `⌘S` on a
//! Mac, and `/keys` and the palette beside them still spelled `Cmd+T` — two
//! vocabularies for one key on one screen. The tables keep the portable
//! spelling (the README parity tests read them, and Windows and Linux draw
//! them as they are); this is the one place a Mac's glyphs are put in, at
//! draw time.
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

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
    if !cfg!(target_os = "macos") || !k.contains('+') {
        return k;
    }
    static MEMO: OnceLock<Mutex<HashMap<&'static str, &'static str>>> = OnceLock::new();
    let mut memo = MEMO
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    memo.entry(k)
        .or_insert_with(|| Box::leak(mac(k).into_boxed_str()))
}

/// A sentence as this platform writes the chords in it — a status flash or
/// a toast (`no shell here — press ⌘T to open one`). Borrowed untouched off
/// a Mac, and wherever there is no chord to rewrite.
pub(crate) fn prose(s: &str) -> std::borrow::Cow<'_, str> {
    match cfg!(target_os = "macos") && s.contains('+') {
        true => std::borrow::Cow::Owned(mac(s)),
        false => std::borrow::Cow::Borrowed(s),
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

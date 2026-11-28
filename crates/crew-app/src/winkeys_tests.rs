use super::*;
use winit::keyboard::SmolStr;

const ALT: ModifiersState = ModifiersState::ALT;
const CTRL: ModifiersState = ModifiersState::CONTROL;
const NONE: ModifiersState = ModifiersState::empty();

fn ch(s: &str) -> Key {
    Key::Character(SmolStr::new(s))
}

fn alt(s: &str) -> Option<WinChord> {
    chord(&ch(s), None, ALT)
}

/// Every Mac Cmd chord crew has, with Alt for Cmd.
#[test]
fn alt_is_cmd() {
    for k in [
        "a", "c", "d", "e", "f", "g", "i", "j", "k", "n", "o", "p", "s", "t", "v", "w", "z", ",",
        ".", ";", "/", "[", "]", "=", "-", "0",
    ] {
        assert_eq!(alt(k), Some(WinChord::Cmd(k)), "Alt+{k}");
    }
}

/// A layout that reports a capital still names the letter.
#[test]
fn a_capital_is_its_letter() {
    assert_eq!(alt("D"), Some(WinChord::Cmd("d")));
}

/// The Mac's three-key chords, each made two keys.
#[test]
fn three_key_chords_are_two() {
    assert_eq!(alt("u"), Some(WinChord::Cmd("T")), "reopen");
    assert_eq!(alt("x"), Some(WinChord::Cmd("E")), "Explorer");
    assert_eq!(alt("m"), Some(WinChord::Cmd("M")), "move to tab");
    assert_eq!(alt("l"), Some(WinChord::Theme));
    assert_eq!(alt("b"), Some(WinChord::Gradient));
    assert_eq!(alt("h"), Some(WinChord::Focus));
    assert_eq!(alt("y"), Some(WinChord::MdSource));
    assert_eq!(alt("q"), Some(WinChord::Quit));
}

#[test]
fn named_keys() {
    let n = |k, m| chord(&Key::Named(k), None, m);
    assert_eq!(n(NamedKey::PageUp, CTRL), Some(WinChord::Tab(-1)));
    assert_eq!(n(NamedKey::PageDown, CTRL), Some(WinChord::Tab(1)));
    assert_eq!(n(NamedKey::PageUp, ALT), Some(WinChord::CmdJump(true)));
    assert_eq!(n(NamedKey::PageDown, ALT), Some(WinChord::CmdJump(false)));
    assert_eq!(n(NamedKey::Home, ALT), Some(WinChord::Screen(-1)));
    assert_eq!(n(NamedKey::End, ALT), Some(WinChord::Screen(1)));
    assert_eq!(n(NamedKey::F11, NONE), Some(WinChord::Maximize));
    assert_eq!(
        n(NamedKey::Enter, ALT),
        Some(WinChord::Cmd("")),
        "inbox send"
    );
    // Plain PgUp/Home and a lone Alt+F11 stay the pane's.
    assert_eq!(n(NamedKey::PageUp, NONE), None);
    assert_eq!(n(NamedKey::Home, NONE), None);
    assert_eq!(n(NamedKey::F11, ALT), None);
}

/// Alt+digit is pane N, Ctrl+digit tab N — read off the physical key, so a
/// layout whose digit row types `&é"` without Shift (AZERTY) still counts.
#[test]
fn digits_by_physical_key() {
    assert_eq!(
        chord(&ch("3"), Some(KeyCode::Digit3), ALT),
        Some(WinChord::Cmd("3"))
    );
    assert_eq!(
        chord(&ch("3"), Some(KeyCode::Digit3), CTRL),
        Some(WinChord::TabN(3))
    );
    assert_eq!(
        chord(&ch("&"), Some(KeyCode::Digit1), ALT),
        Some(WinChord::Cmd("1"))
    );
    assert_eq!(chord(&ch("1"), Some(KeyCode::Digit1), NONE), None);
}

/// The OS's keys, AltGr, the Windows key and three-key presses are never crew's.
#[test]
fn not_crews() {
    let n = |k| chord(&Key::Named(k), None, ALT);
    assert_eq!(n(NamedKey::F4), None, "Alt+F4 closes the window");
    assert_eq!(n(NamedKey::Tab), None, "Alt+Tab switches apps");
    assert_eq!(n(NamedKey::Space), None, "Alt+Space is the window menu");
    assert_eq!(chord(&ch("d"), None, CTRL | ALT), None, "AltGr types");
    assert_eq!(chord(&ch("d"), None, ModifiersState::SUPER), None);
    assert_eq!(chord(&ch("d"), None, ALT | ModifiersState::SHIFT), None);
    assert_eq!(chord(&ch("d"), None, CTRL), None, "Ctrl+D is the shell's");
    assert_eq!(chord(&ch("d"), None, NONE), None);
}

/// A letter crew does not use stays the terminal's Meta.
#[test]
fn unused_alt_letters_pass_through() {
    assert_eq!(alt("r"), None);
}

/// A label part (`Alt+U`, `Ctrl+PgUp`, `F11`, `Ctrl+1 … 9`) as a key press.
fn press(part: &str) -> (Key, Option<KeyCode>, ModifiersState) {
    let (mods, key) = match part.split_once('+') {
        Some(("Alt", k)) => (ALT, k),
        Some(("Ctrl", k)) => (CTRL, k),
        Some((m, _)) => panic!("{part}: {m}+ is not one of the two modifiers"),
        None => (NONE, part),
    };
    let named = |n| (Key::Named(n), None, mods);
    match key {
        "PgUp" => named(NamedKey::PageUp),
        "PgDn" => named(NamedKey::PageDown),
        "Home" => named(NamedKey::Home),
        "End" => named(NamedKey::End),
        "F11" => named(NamedKey::F11),
        "1 \u{2026} 9" => (ch("1"), Some(KeyCode::Digit1), mods),
        k => (ch(&k.to_lowercase()), None, mods),
    }
}

/// Every remapped label is at most two keys AND names a key `chord` answers
/// to — the label and the key cannot drift apart.
#[test]
fn remaps_name_keys_that_work() {
    for (mac, win) in REMAPS {
        for part in win.split(" / ") {
            let (key, code, mods) = press(part);
            assert!(
                chord(&key, code, mods).is_some(),
                "{mac} \u{2192} {win}: {part} does nothing"
            );
        }
    }
}

/// No two Alt letters make the same chord, and none shadows the four that
/// are not Cmd chords.
#[test]
fn the_table_has_no_duplicates() {
    let mut keys: Vec<_> = super::ALT.iter().map(|(k, _)| *k).collect();
    keys.extend(["l", "b", "h", "y"]);
    let n = keys.len();
    keys.sort_unstable();
    keys.dedup();
    assert_eq!(keys.len(), n);
}

/// A label and its key come from one table: every `REMAPS` row, written off a
/// Mac, is its Windows spelling, and pressing that spelling makes the chord
/// the Mac's spelling makes (`Alt+U` is what Cmd+Shift+T is).
#[test]
fn a_remapped_label_binds_the_chord_it_replaces() {
    use crate::chordglyph::offmac;
    let cmd = |s| Some(vec![WinChord::Cmd(s)]);
    let want: [(&str, Option<Vec<WinChord>>); 13] = [
        ("Cmd+Shift+T", cmd("T")),
        ("Cmd+Shift+E", cmd("E")),
        ("Cmd+Shift+M", cmd("M")),
        ("Cmd+Shift+C", cmd("C")),
        ("Cmd+M", Some(vec![WinChord::Maximize])),
        (
            "Cmd+{ / Cmd+}",
            Some(vec![WinChord::Tab(-1), WinChord::Tab(1)]),
        ),
        ("Cmd+Opt+1 \u{2026} 9", Some(vec![WinChord::TabN(1)])),
        (
            "Cmd+Opt+\u{2191}\u{2193}",
            Some(vec![WinChord::CmdJump(true), WinChord::CmdJump(false)]),
        ),
        (
            "Cmd+Opt+\u{2190}\u{2192}",
            Some(vec![WinChord::Screen(-1), WinChord::Screen(1)]),
        ),
        ("Ctrl+Shift+L", Some(vec![WinChord::Theme])),
        ("Ctrl+Shift+G", Some(vec![WinChord::Gradient])),
        ("Ctrl+Shift+F", Some(vec![WinChord::Focus])),
        ("Ctrl+Shift+M", Some(vec![WinChord::MdSource])),
    ];
    assert_eq!(want.len(), REMAPS.len(), "a new row needs its chord here");
    for ((mac, win), (m, chords)) in REMAPS.iter().zip(want) {
        assert_eq!(*mac, m);
        assert_eq!(offmac(mac), *win, "the label is the table's");
        let got: Vec<_> = win
            .split(" / ")
            .map(|p| {
                let (key, code, mods) = press(p);
                chord(&key, code, mods).unwrap_or_else(|| panic!("{p} does nothing"))
            })
            .collect();
        assert_eq!(Some(got), chords, "{mac} \u{2192} {win}");
    }
}

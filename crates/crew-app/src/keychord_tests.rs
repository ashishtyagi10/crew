use super::*;
use winit::keyboard::ModifiersState;

#[test]
fn is_compact_chord_matches_ctrl_o_only() {
    assert!(is_compact_chord(
        &Key::Character("o".into()),
        ModifiersState::CONTROL
    ));
    // Case-insensitive, matching how Ctrl+Shift+M's own match is written.
    assert!(is_compact_chord(
        &Key::Character("O".into()),
        ModifiersState::CONTROL
    ));
}

#[test]
fn is_compact_chord_requires_control() {
    assert!(!is_compact_chord(
        &Key::Character("o".into()),
        ModifiersState::empty()
    ));
}

#[test]
fn is_compact_chord_rejects_other_letters() {
    assert!(!is_compact_chord(
        &Key::Character("k".into()),
        ModifiersState::CONTROL
    ));
}

#[test]
fn arrow_dir_maps_the_four_arrows_and_nothing_else() {
    use crate::panedir::Dir;
    assert_eq!(arrow_dir(&Key::Named(NamedKey::ArrowLeft)), Some(Dir::Left));
    assert_eq!(
        arrow_dir(&Key::Named(NamedKey::ArrowRight)),
        Some(Dir::Right)
    );
    assert_eq!(arrow_dir(&Key::Named(NamedKey::ArrowUp)), Some(Dir::Up));
    assert_eq!(arrow_dir(&Key::Named(NamedKey::ArrowDown)), Some(Dir::Down));
    assert_eq!(arrow_dir(&Key::Named(NamedKey::Enter)), None);
    assert_eq!(arrow_dir(&Key::Character("k".into())), None);
}

#[test]
fn is_compact_chord_rejects_named_keys() {
    assert!(!is_compact_chord(
        &Key::Named(NamedKey::Escape),
        ModifiersState::CONTROL
    ));
}

#[test]
fn off_a_mac_plain_ctrl_is_cmd_except_what_a_terminal_keeps() {
    let (c, cs) = (
        ModifiersState::CONTROL,
        ModifiersState::CONTROL | ModifiersState::SHIFT,
    );
    let ch = |s: &str| Key::Character(s.into());
    let at = |k: &str, m, typed| stand_in(&ch(k), m, false, typed);
    // The user's chord: Ctrl+T opens a pane, in a terminal too.
    assert_eq!(at("t", c, true).as_deref(), Some("t"));
    assert_eq!(at("t", c, false).as_deref(), Some("t"));
    assert_eq!(at("w", c, true).as_deref(), Some("w"));
    assert_eq!(at("1", c, true).as_deref(), Some("1"));
    // A terminal keeps its own: ^C, ^D, ^R, line editing, Enter, Tab, Esc.
    for k in [
        "c", "d", "z", "r", "l", "a", "e", "k", "u", "m", "j", "i", "[",
    ] {
        assert_eq!(at(k, c, true), None, "{k} stays with the program");
    }
    assert_eq!(
        at("c", c, false).as_deref(),
        Some("c"),
        "outside one it is crew's"
    );
    // Ctrl+Shift reaches crew everywhere, and shifts what a Mac shifts.
    assert_eq!(at("c", cs, true).as_deref(), Some("c"));
    assert_eq!(
        at("t", cs, true).as_deref(),
        Some("T"),
        "reopen, as a browser"
    );
    assert_eq!(at("[", cs, false).as_deref(), Some("{"));
    assert_eq!(at("1", cs, true).as_deref(), Some("1"));
    // Ctrl+Shift chords crew already means something by keep that meaning.
    for k in ["l", "g", "f", "m"] {
        assert_eq!(at(k, cs, false), None, "{k}");
    }
    // A Mac has its Cmd key; AltGr is Ctrl+Alt; nothing without Ctrl.
    assert_eq!(stand_in(&ch("t"), c, true, false), None);
    assert_eq!(at("t", c | ModifiersState::ALT, false), None);
    assert_eq!(at("t", ModifiersState::SHIFT, false), None);
    assert!(!is_compact_chord(&ch("O"), cs), "Ctrl+Shift+O is Cmd+O");
}

#[test]
fn off_a_mac_the_grid_walks_with_alt_and_never_with_altgr() {
    use crate::ctrlchord::grid_held;
    assert!(grid_held(ModifiersState::ALT, false));
    assert!(grid_held(
        ModifiersState::ALT | ModifiersState::SHIFT,
        false
    ));
    assert!(!grid_held(
        ModifiersState::ALT | ModifiersState::CONTROL,
        false
    ));
    assert!(
        !grid_held(ModifiersState::ALT, true),
        "a Mac's Option moves by word"
    );
    assert!(grid_held(ModifiersState::SUPER, true));
}

/// Ctrl+P is a shell's previous line: off a Mac, Cmd+P's go-to-file must not take it from a
/// terminal or a typed line — and still reaches go-to-file everywhere else.
#[test]
fn ctrl_p_stays_the_shells_off_a_mac() {
    assert!(!plain("p", true));
    assert!(plain("p", false));
}

/// Ctrl+N is the shell's next line, and fzf's and vim's completion menu's
/// down: off a Mac it reached Cmd+N and opened a new window instead. It is
/// the terminal's now — and still a new window outside one, or with Shift.
#[test]
fn ctrl_n_stays_the_shells_off_a_mac() {
    let ch = Key::Character("n".into());
    assert_eq!(stand_in(&ch, ModifiersState::CONTROL, false, true), None);
    let outside = stand_in(&ch, ModifiersState::CONTROL, false, false);
    assert_eq!(outside.as_deref(), Some("n"));
    let shifted = ModifiersState::CONTROL | ModifiersState::SHIFT;
    assert_eq!(stand_in(&ch, shifted, false, true).as_deref(), Some("n"));
}

/// Off a Mac, Ctrl+N in Far's command line or a settings field is no chord
/// (Far counts as typing) — and no letter either: it typed a stray `n`.
#[test]
fn a_ctrl_letter_is_no_text_to_type() {
    let ch = |s: &str| Key::Character(s.into());
    assert!(ctrl_letter(&ch("n"), ModifiersState::CONTROL));
    assert!(ctrl_letter(
        &ch("N"),
        ModifiersState::CONTROL | ModifiersState::SHIFT
    ));
    assert!(
        !ctrl_letter(&ch("n"), ModifiersState::empty()),
        "a plain letter types"
    );
    let altgr = ModifiersState::CONTROL | ModifiersState::ALT;
    assert!(
        !ctrl_letter(&ch("q"), altgr),
        "AltGr+Q is @ on a German layout"
    );
    assert!(!ctrl_letter(&ch("@"), ModifiersState::CONTROL));
}

//! Type-to-find: letters jump the tree's cursor to a name, the same letter again steps on, the
//! prefix shows in the rule and goes after a pause, an Esc, a Backspace to empty or a move.
use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::app::CrewApp;
use crate::explorer::tests::tree;

/// Rows, folders first: 0 the root, 1 scripts, 2 src, 3 static, 4 alpha.txt, 5 ok.md,
/// 6 okjk.md.
const FILES: &[&str] = &[
    "scripts/",
    "src/",
    "static/",
    "alpha.txt",
    "ok.md",
    "okjk.md",
];

fn app(name: &str) -> CrewApp {
    let root = tree(name, FILES);
    let mut a = CrewApp {
        geo_override: Some((8.0, 17.0, 1280.0, 720.0, 1.0)),
        cwd: root,
        ..Default::default()
    };
    a.config.show_nav = false;
    at(0);
    a.explorer_keys_toggle();
    assert!(a.tree_has_keys());
    a
}

/// Set the clock the prefix ages by.
fn at(ms: u64) {
    super::TEST_NOW.with(|t| t.set(Some(ms)));
}

fn typ(a: &mut CrewApp, s: &str) -> bool {
    a.explorer_tree_key(&Key::Character(s.into()), true)
}

fn press(a: &mut CrewApp, k: NamedKey) -> bool {
    a.explorer_tree_key(&Key::Named(k), true)
}

/// The cursor row's name.
fn on(a: &CrewApp) -> String {
    let items = a.sidebar.explorer.items(&a.explorer_root());
    items[a.sidebar.explorer.keys.expect("held").row]
        .name
        .clone()
}

fn prefix(a: &CrewApp) -> Option<String> {
    let live = a.sidebar.explorer.find.live(super::now());
    live.map(|(p, _)| p.to_string())
}

#[test]
fn letters_jump_and_the_same_letter_cycles() {
    let _g = crate::app::theme_test_guard();
    let mut a = app("fjump");
    assert!(typ(&mut a, "s"));
    assert_eq!(on(&a), "scripts");
    assert!(typ(&mut a, "r"));
    assert_eq!(on(&a), "src", "s, r: src");
    assert_eq!(prefix(&a).as_deref(), Some("sr"));
    let mut a = app("fcycle");
    for want in ["scripts", "src", "static", "scripts"] {
        typ(&mut a, "s");
        assert_eq!(on(&a), want, "s again steps on, and wraps round");
    }
    assert_eq!(prefix(&a).as_deref(), Some("s"));
    typ(&mut a, "S");
    assert_eq!(on(&a), "src", "any case");
}

#[test]
fn j_and_k_move_until_a_prefix_then_extend_it() {
    let _g = crate::app::theme_test_guard();
    let mut a = app("fjk");
    typ(&mut a, "j");
    assert_eq!(on(&a), "scripts", "j moves");
    typ(&mut a, "k");
    assert_eq!(a.sidebar.explorer.keys.unwrap().row, 0, "k moves");
    assert_eq!(prefix(&a), None);
    typ(&mut a, "o");
    typ(&mut a, "k");
    assert_eq!(on(&a), "ok.md", "k extends: ok");
    typ(&mut a, "j");
    assert_eq!(on(&a), "okjk.md", "j extends: okj");
    assert_eq!(prefix(&a).as_deref(), Some("okj"));
}

#[test]
fn backspace_shortens_then_ends_the_find() {
    let _g = crate::app::theme_test_guard();
    let mut a = app("fback");
    for s in ["o", "k", "j"] {
        typ(&mut a, s);
    }
    assert!(press(&mut a, NamedKey::Backspace));
    assert_eq!(prefix(&a).as_deref(), Some("ok"));
    assert_eq!(
        on(&a),
        "okjk.md",
        "a shorter prefix still matches where it stands"
    );
    press(&mut a, NamedKey::Backspace);
    assert!(press(&mut a, NamedKey::Backspace), "to empty");
    assert_eq!(prefix(&a), None);
    assert!(a.tree_has_keys());
    typ(&mut a, "k");
    assert_eq!(on(&a), "ok.md", "k is a motion again");
}

/// Nothing starts with it: the cursor stays, the prefix stays, the rule shows it in danger ink.
#[test]
fn no_match_keeps_the_cursor_and_shows_it_muted() {
    let _g = crate::app::theme_test_guard();
    let mut a = app("fnone");
    typ(&mut a, "a");
    assert_eq!(on(&a), "alpha.txt");
    assert!(typ(&mut a, "q"));
    assert_eq!(on(&a), "alpha.txt");
    assert_eq!(prefix(&a).as_deref(), Some("aq"));
    let rule = |a: &CrewApp| {
        let cells = a.explorer_cells(40, 10);
        let mut row: Vec<_> = cells.into_iter().filter(|c| c.row == 0).collect();
        row.sort_by_key(|c| c.col);
        row
    };
    let cells = rule(&a);
    let text: String = cells.iter().map(|c| c.c).collect();
    assert!(text.contains("EXPLORER find: aq"), "{text}");
    let q = cells.iter().rfind(|c| c.c == 'q').unwrap();
    assert_eq!(q.fg, crew_theme::readable::danger(crew_theme::theme()));
    press(&mut a, NamedKey::Backspace);
    let q = rule(&a).into_iter().rfind(|c| c.c == 'a').unwrap();
    assert_eq!(q.fg, crew_theme::theme().ink, "found: in ink");
}

/// A second of quiet ends the find — read on the next key, and by the poll loop so the legend
/// is repainted away without one.
#[test]
fn the_prefix_ages_out() {
    let _g = crate::app::theme_test_guard();
    let mut a = app("fage");
    at(1000);
    typ(&mut a, "s");
    at(1999);
    assert!(!a.explorer_find_expire(1999));
    assert_eq!(prefix(&a).as_deref(), Some("s"));
    typ(&mut a, "r");
    assert_eq!(on(&a), "src", "each key restarts the pause");
    at(2999);
    typ(&mut a, "j");
    assert_eq!(on(&a), "static", "aged out: j moves");
    assert_eq!(prefix(&a), None);
    typ(&mut a, "s");
    assert!(!a.explorer_find_expire(3998));
    assert!(
        a.explorer_find_expire(3999),
        "aged out: cleared, one repaint"
    );
    assert!(!a.explorer_find_expire(3999), "and only one");
}

#[test]
fn esc_clears_the_find_then_gives_the_keys_back() {
    let _g = crate::app::theme_test_guard();
    let mut a = app("fesc");
    typ(&mut a, "s");
    assert!(press(&mut a, NamedKey::Escape));
    assert_eq!(prefix(&a), None);
    assert!(a.tree_has_keys(), "the first Esc ends the find");
    assert!(press(&mut a, NamedKey::Escape));
    assert!(!a.tree_has_keys(), "the second gives the keys back");
}

/// A movement key ends the find; Shift on its own (a capital coming) does not; a letter with
/// Cmd held is a chord, not typing.
#[test]
fn moves_end_it_shift_does_not_chords_are_not_typing() {
    let _g = crate::app::theme_test_guard();
    let mut a = app("fmove");
    typ(&mut a, "s");
    assert!(press(&mut a, NamedKey::Shift));
    assert_eq!(prefix(&a).as_deref(), Some("s"));
    press(&mut a, NamedKey::ArrowDown);
    assert_eq!(on(&a), "src");
    assert_eq!(prefix(&a), None, "a move ends it");
    typ(&mut a, "s");
    assert_eq!(on(&a), "src", "a fresh find matches where it stands");
    a.mods = ModifiersState::SUPER.into();
    assert!(!typ(&mut a, "p"), "Cmd+P goes on");
    assert_eq!(prefix(&a), None);
}

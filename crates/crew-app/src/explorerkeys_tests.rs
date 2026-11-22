//! The tree from the keyboard: Cmd+Shift+E takes the keys, the arrows walk and open, and any
//! other focus change gives them back.
use winit::keyboard::{Key, NamedKey};

use crate::app::CrewApp;
use crate::explorer::tests::tree;

fn app(root: &std::path::Path) -> CrewApp {
    let mut a = CrewApp {
        geo_override: Some((8.0, 17.0, 1280.0, 720.0, 1.0)),
        cwd: root.to_path_buf(),
        ..Default::default()
    };
    a.config.show_nav = false;
    a.input.focused = true;
    a
}

fn press(a: &mut CrewApp, k: NamedKey) -> bool {
    a.explorer_tree_key(&Key::Named(k), true)
}

fn row(a: &CrewApp) -> usize {
    a.sidebar.explorer.keys.expect("the tree has the keys").row
}

/// The chord opens a hidden nav on the Explorer and takes the keys from the bar; again, it
/// gives them back.
#[test]
fn the_chord_opens_the_explorer_and_takes_the_keys() {
    let _g = crate::app::theme_test_guard();
    let root = tree("kchord", &["a.txt"]);
    let mut a = app(&root);
    a.explorer_keys_toggle();
    assert!(a.config.show_nav && !a.config.nav_collapsed);
    assert_eq!(a.config.nav_section, "explorer");
    assert!(!a.input.focused, "the bar gave up the keys");
    assert!(a.tree_has_keys());
    a.explorer_keys_toggle();
    assert!(!a.tree_has_keys());
}

/// Down walks, → opens a folder and steps into it, Enter on a file previews it and keeps the
/// keys, ← steps to the parent then closes it.
#[test]
fn arrows_walk_open_and_preview() {
    let _g = crate::app::theme_test_guard();
    let root = tree("kwalk", &["docs/guide.md", "notes.txt"]);
    let mut a = app(&root);
    // Focus somewhere the preview will not land, so the hand-over is seen.
    a.focused = 5;
    a.explorer_keys_toggle();
    // Row 0 the root, 1 docs (closed), 2 notes.txt.
    assert!(press(&mut a, NamedKey::ArrowDown));
    assert_eq!(row(&a), 1);
    press(&mut a, NamedKey::ArrowRight);
    assert_eq!(a.sidebar.explorer.items(&root).len(), 4, "docs opened");
    press(&mut a, NamedKey::ArrowRight);
    assert_eq!(row(&a), 2, "into docs: guide.md");
    press(&mut a, NamedKey::Enter);
    assert_eq!(a.panes.len(), 1, "previewed in a viewer");
    assert_eq!(a.sidebar.explorer.picked, Some(root.join("docs/guide.md")));
    assert!(a.tree_has_keys(), "browsing goes on in the tree");
    press(&mut a, NamedKey::ArrowLeft);
    assert_eq!(row(&a), 1, "← from a file: its folder");
    press(&mut a, NamedKey::ArrowLeft);
    assert_eq!(
        a.sidebar.explorer.items(&root).len(),
        3,
        "← on an open folder closes it"
    );
    press(&mut a, NamedKey::End);
    assert_eq!(row(&a), 2);
    press(&mut a, NamedKey::Home);
    assert_eq!(row(&a), 0);
}

/// Esc gives the keys back; so does any focus change made elsewhere — the bar taking them,
/// another pane — with no path having to know the tree exists. A key the tree has no use for
/// ends the mode and is not swallowed.
#[test]
fn the_keys_go_back_on_esc_or_any_focus_change() {
    let _g = crate::app::theme_test_guard();
    let root = tree("kback", &["a.txt"]);
    let mut a = app(&root);
    a.explorer_keys_toggle();
    assert!(press(&mut a, NamedKey::Escape));
    assert!(!a.tree_has_keys());
    a.explorer_keys_toggle();
    a.input.focused = true;
    assert!(!a.tree_has_keys(), "Cmd+I took them");
    a.input.focused = false;
    a.explorer_keys_toggle();
    a.focused += 1;
    assert!(!a.tree_has_keys(), "another pane took them");
    a.focused -= 1;
    a.explorer_keys_toggle();
    assert!(
        !a.explorer_tree_key(&Key::Character("x".into()), true),
        "x goes on to the pane"
    );
    assert!(!a.tree_has_keys());
}

/// While the tree holds the keys its cursor row is one bar across the panel.
#[test]
fn the_cursor_row_is_a_bar() {
    let _g = crate::app::theme_test_guard();
    let root = tree("kmark", &["a.txt"]);
    let mut a = app(&root);
    a.explorer_keys_toggle();
    press(&mut a, NamedKey::ArrowDown);
    let cells = a.explorer_cells(30, 10);
    let (_, fill) = crate::segment::inked(crate::palette::accent());
    let barred: Vec<u16> = (cells.iter())
        .filter(|c| c.bg == fill)
        .map(|c| c.row)
        .collect();
    assert_eq!(barred.len(), 30, "every column of one row");
    assert!(barred.iter().all(|&r| r == crate::explorerdraw::TOP + 1));
    a.sidebar.explorer.keys = None;
    let quiet = a.explorer_cells(30, 10);
    assert!(
        quiet.iter().all(|c| c.bg != fill),
        "no bar without the keys"
    );
}

/// Lost once, lost for good: the bar taking the keys and giving them back, or a click off
/// the tree, does not re-arm it.
#[test]
fn a_lost_hold_does_not_come_back() {
    let _g = crate::app::theme_test_guard();
    let root = tree("klost", &["a.txt"]);
    let mut a = app(&root);
    a.explorer_keys_toggle();
    a.input.focused = true;
    let _ = a.build_frame();
    a.input.focused = false;
    assert!(!a.tree_has_keys(), "Cmd+I then Esc gives the pane its keys");
    a.explorer_keys_toggle();
    a.cursor = (900.0, 400.0);
    let _ = a.docked_press();
    assert!(a.sidebar.explorer.keys.is_none(), "a click off the tree");
}

/// `/nav explorer` — the keyboard's way in on every platform — hands the tree the keys.
#[test]
fn nav_explorer_typed_gives_the_tree_the_keys() {
    let _g = crate::app::theme_test_guard();
    let root = tree("knav", &["a.txt"]);
    let mut a = app(&root);
    a.nav_command("explorer");
    assert!(a.tree_has_keys());
}

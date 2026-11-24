//! The tree follows the focused viewer: opened down to its file, marked, scrolled to it — once
//! per change, so a scroll afterwards is the user's.
use crate::app::CrewApp;
use crate::explorer::tests::tree;
use crate::explorerhit::tests::{app, at};

/// `a` with a viewer on `file` focused, opened the way `/view` and Cmd+P open one.
fn viewing(a: &mut CrewApp, file: &std::path::Path) {
    a.open_view_here(&file.to_string_lossy());
    assert!(
        matches!(&a.panes[a.focused].content, crate::pane::PaneContent::View(v) if v.path == file),
        "the viewer has focus"
    );
}

fn listed(a: &CrewApp, root: &std::path::Path, file: &std::path::Path) -> Option<usize> {
    let items = a.sidebar.explorer.items(root);
    items.iter().position(|i| i.path == file)
}

/// A file three folders down opens the tree to it, marks it, and is drawn in the panel; the
/// chord then starts the keyboard cursor on it.
#[test]
fn a_viewer_opened_elsewhere_opens_the_tree_down_to_its_file() {
    let _g = crate::app::theme_test_guard();
    let root = tree("reveal", &["a/b/c.rs", "z.txt"]);
    let file = root.join("a/b/c.rs");
    let mut a = app(&root);
    viewing(&mut a, &file);
    assert_eq!(a.explorer_root(), root, "the viewer kept the tree's root");
    assert_eq!(listed(&a, &root, &file), None, "closed before the frame");
    at(&mut a, "c.rs");
    let row = listed(&a, &root, &file).expect("c.rs is a row of the tree");
    assert_eq!(a.sidebar.explorer.picked.as_deref(), Some(file.as_path()));
    a.explorer_keys_toggle();
    assert_eq!(a.sidebar.explorer.keys.map(|k| k.row), Some(row));
}

/// Far down a long folder, the tree scrolls the file's row to the middle of the panel — its
/// neighbours below are drawn too, not cut off at the edge.
#[test]
fn a_file_far_down_is_scrolled_into_view() {
    let _g = crate::app::theme_test_guard();
    let names: Vec<String> = (0..120).map(|i| format!("f/file{i:03}.txt")).collect();
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let root = tree("revealfar", &refs);
    let mut a = app(&root);
    viewing(&mut a, &root.join("f/file060.txt"));
    a.build_frame();
    assert!(a.sidebar.explorer.scroll > 0, "scrolled");
    at(&mut a, "file060.txt");
    at(&mut a, "file066.txt");
}

/// After the reveal the tree is the user's: scrolled back to the top it stays there, until the
/// focused file changes.
#[test]
fn a_scroll_after_the_reveal_is_kept_until_the_file_changes() {
    let _g = crate::app::theme_test_guard();
    let names: Vec<String> = (0..60).map(|i| format!("file{i:02}.txt")).collect();
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let root = tree("revealkeep", &refs);
    let mut a = app(&root);
    viewing(&mut a, &root.join("file55.txt"));
    a.build_frame();
    a.sidebar.explorer.scroll = 0;
    a.build_frame();
    assert_eq!(a.sidebar.explorer.scroll, 0, "no snap back");
    viewing(&mut a, &root.join("file58.txt"));
    a.build_frame();
    assert!(a.sidebar.explorer.scroll > 0, "a new file is revealed");
}

/// A file outside the tree's root leaves the tree as it was.
#[test]
fn a_file_outside_the_root_changes_nothing() {
    let _g = crate::app::theme_test_guard();
    let root = tree("revealin", &["a/x.rs"]);
    let other = tree("revealout", &["a/y.rs"]);
    let mut a = app(&root);
    a.sidebar.explorer.root.replace(Some(root.clone()));
    viewing(&mut a, &other.join("a/y.rs"));
    let before = a.sidebar.explorer.items(&root).len();
    a.build_frame();
    assert_eq!(a.sidebar.explorer.picked, None);
    assert_eq!(a.sidebar.explorer.items(&root).len(), before);
}

/// From a hidden nav, the chord opens the Explorer already revealed, the cursor on the file.
#[test]
fn the_chord_from_a_closed_nav_starts_on_the_file() {
    let _g = crate::app::theme_test_guard();
    let root = tree("revealchord", &["a/b/c.rs", "a/d.rs"]);
    let file = root.join("a/b/c.rs");
    let mut a = app(&root);
    a.config.show_nav = false;
    viewing(&mut a, &file);
    a.build_frame();
    a.explorer_keys_toggle();
    let row = listed(&a, &root, &file).expect("revealed by the chord");
    assert_eq!(a.sidebar.explorer.keys.map(|k| k.row), Some(row));
}

/// The Explorer over crew's own crate, a viewer on a nested file focused: the tree opened down
/// to it and its row marked.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn explorer_shot_revealed() {
    use crew_theme::ThemeId;
    let _g = crate::app::theme_test_guard();
    for id in [ThemeId::TokyoNight, ThemeId::Porcelain] {
        let name = format!("explorer-reveal-{id:?}").to_lowercase();
        let prep = |a: &mut CrewApp| {
            a.config.show_nav = true;
            a.config.nav_collapsed = false;
            a.config.nav_section = "explorer".into();
            let file = a.explorer_root().join("src/viewpane/pane.rs");
            a.open_view_here(&file.to_string_lossy());
            a.input.focused = false;
        };
        let opacity = crate::tubesheer::sheer(id.theme());
        crate::crtglassshot_tests::window_with(&name, id, opacity, &[], &prep);
    }
}

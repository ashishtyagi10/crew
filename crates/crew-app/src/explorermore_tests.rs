//! The "… n more" line closing a capped folder opens that folder in Far — by a click on the
//! drawn row, or Enter from the keyboard — and lights under the pointer like any row it opens.
use winit::keyboard::{Key, NamedKey};

use crate::explorer::CAP;
use crate::explorerhit::tests::{app, at};
use crate::pane::PaneContent;

/// A folder of `CAP + 3` files: the tree keeps `CAP` and says "… 3 more".
fn capped(name: &str) -> std::path::PathBuf {
    let files: Vec<String> = (0..CAP + 3).map(|i| format!("f{i:05}")).collect();
    let refs: Vec<&str> = files.iter().map(String::as_str).collect();
    crate::explorer::tests::tree(name, &refs)
}

/// Where Far's active panel stands in pane `i`.
fn far_dir(a: &crate::app::CrewApp, i: usize) -> Option<std::path::PathBuf> {
    match &a.panes.get(i)?.content {
        PaneContent::Far(f) => f.active_local_dir(),
        _ => None,
    }
}

#[test]
fn a_click_on_n_more_opens_the_folder_in_far() {
    let _g = crate::app::theme_test_guard();
    let root = capped("morefar");
    let mut a = app(&root);
    a.sidebar.explorer.scroll = usize::MAX; // the last row on top, where it can be clicked
    a.cursor = at(&mut a, "3 more");
    assert!(a.explorer_press());
    assert_eq!(a.panes.len(), 1, "a Far pane opened");
    assert_eq!(a.focused, 0);
    assert_eq!(far_dir(&a, 0), Some(root.clone()));
    // A second click goes to the Far pane already open, not a new one.
    a.cursor = at(&mut a, "3 more");
    a.explorer_press();
    assert_eq!(a.panes.len(), 1);
}

#[test]
fn enter_on_n_more_opens_far_and_hands_it_the_keys() {
    let _g = crate::app::theme_test_guard();
    let root = capped("morekeys");
    let mut a = app(&root);
    a.config.show_nav = false;
    a.explorer_keys_toggle();
    assert!(a.explorer_tree_key(&Key::Named(NamedKey::End), true));
    assert!(a.explorer_tree_key(&Key::Named(NamedKey::Enter), true));
    let far = a.panes.len() - 1;
    assert_eq!(a.focused, far);
    assert_eq!(far_dir(&a, far), Some(root));
    assert!(!a.tree_has_keys(), "Far took focus, and the keys with it");
}

/// Muted at rest — it is not a file — but the accent under the pointer, as every row a click
/// opens is.
#[test]
fn n_more_lights_under_the_pointer() {
    let _g = crate::app::theme_test_guard();
    let root = capped("morelit");
    let items = crate::explorer::Explorer::default().items(&root);
    let last = items.len() - 1;
    let ink = |hover| {
        let marks = crate::explorerdraw::Marks {
            hover,
            ..Default::default()
        };
        let cells = crate::explorerdraw::cells(&items, last, marks, (30, 10));
        cells
            .iter()
            .find(|c| c.row >= crate::explorerdraw::TOP && c.c == 'm')
            .expect("'more' drawn")
            .fg
    };
    let t = crew_theme::theme();
    assert_eq!(ink(None), t.text_muted);
    assert_ne!(ink(Some(last)), t.text_muted);
}

//! Pinned folders in the app: drawn as roots after the pane's, opened and walked like it,
//! unpinned by the `×`, by Delete, and by `/nav explorer remove`.
use winit::keyboard::{Key, NamedKey};

use crate::app::CrewApp;
use crate::explorer::tests::tree;
use crate::explorerhit::tests::{app, at};

fn said(a: &CrewApp) -> String {
    a.status.clone().map(|(m, _)| m).unwrap_or_default()
}

fn names(a: &CrewApp) -> Vec<String> {
    a.explorer_items().iter().map(|i| i.name.clone()).collect()
}

/// A pane root holding `a.txt`, and a folder elsewhere holding `creds/app.properties`.
fn two(name: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let here = tree(&format!("{name}-here"), &["a.txt"]);
    let far = tree(&format!("{name}-far"), &["creds/app.properties"]);
    (here, far.join("creds"))
}

#[test]
fn a_pinned_folder_is_a_root_after_the_panes_and_its_files_open() {
    let _g = crate::app::theme_test_guard();
    let (here, creds) = two("pdraw");
    let mut a = app(&here);
    a.nav_command(&format!("explorer add {}", creds.display()));
    assert_eq!(a.sidebar.explorer.pinned(), vec![creds.clone()]);
    let n = names(&a);
    assert_eq!(n[1..], ["a.txt", "creds", "app.properties"], "{n:?}");
    a.cursor = at(&mut a, "CREDS");
    a.cursor = at(&mut a, "app.properties");
    assert!(a.explorer_press());
    assert_eq!(a.panes.len(), 1, "the file opened in a viewer");
    let file = creds.join("app.properties");
    assert_eq!(a.sidebar.explorer.picked.as_deref(), Some(file.as_path()));
}

#[test]
fn the_panes_own_folder_pinned_is_not_drawn_twice() {
    let _g = crate::app::theme_test_guard();
    let (here, _) = two("psame");
    let mut a = app(&here);
    a.nav_command("explorer add .");
    assert!(
        a.sidebar.explorer.pinned().is_empty(),
        "the root is not pinned"
    );
    assert!(said(&a).contains("already the tree's root"));
    // Pinned while another pane was in front, it still is not drawn twice once it is the root.
    a.sidebar.explorer.pin(&here);
    assert_eq!(a.explorer_roots(), vec![here.clone()]);
}

#[test]
fn a_file_pins_its_folder_and_opens_revealed_under_it() {
    let _g = crate::app::theme_test_guard();
    let (here, creds) = two("pfile");
    let mut a = app(&here);
    let file = creds.join("app.properties");
    a.nav_command(&format!("explorer add {}", file.display()));
    assert_eq!(a.sidebar.explorer.pinned(), vec![creds.clone()]);
    assert!(
        matches!(&a.panes[a.focused].content, crate::pane::PaneContent::View(v) if v.path == file),
        "the file is open"
    );
    at(&mut a, "app.properties");
    assert_eq!(a.sidebar.explorer.picked.as_deref(), Some(file.as_path()));
    assert_eq!(a.explorer_root(), here, "the pane's root stays first");
}

#[test]
fn the_x_shows_on_a_hovered_pinned_root_and_a_click_on_it_unpins() {
    let _g = crate::app::theme_test_guard();
    let (here, creds) = two("px");
    let mut a = app(&here);
    a.sidebar.explorer.pin(&creds);
    let drawn = |a: &mut CrewApp| {
        let scenes = a.build_frame();
        let nav = scenes.iter().find(|s| s.x < 30.0 && s.h > 60.0).unwrap();
        nav.cells.iter().any(|c| c.c == '\u{d7}')
    };
    assert!(!drawn(&mut a), "no × until the root is hovered");
    a.cursor = at(&mut a, "a.txt");
    assert!(!drawn(&mut a), "never on the pane's own tree");
    let name = at(&mut a, "CREDS");
    a.cursor = name;
    let x = at(&mut a, "\u{d7}");
    assert_eq!(x.1, name.1, "the × is on the hovered root's row");
    // A press on the name closes the root, as any root's does — it stays pinned.
    assert!(a.explorer_press());
    assert_eq!(a.sidebar.explorer.pinned(), vec![creds.clone()]);
    assert!(!names(&a).contains(&"app.properties".to_string()));
    a.cursor = x;
    assert!(a.explorer_x_at_cursor().is_some());
    assert!(a.explorer_press());
    assert!(a.sidebar.explorer.pinned().is_empty(), "the × unpinned it");
    assert_eq!(names(&a).len(), 2);
}

#[test]
fn the_keys_walk_across_roots_and_delete_unpins_only_a_pinned_one() {
    let _g = crate::app::theme_test_guard();
    let (here, creds) = two("pkeys");
    let mut a = app(&here);
    a.sidebar.explorer.pin(&creds);
    a.explorer_keys_toggle();
    let key = |a: &mut CrewApp, k| a.explorer_tree_key(&Key::Named(k), true);
    // The pane's own root: Delete is not the tree's, and gives the keys back.
    assert!(!key(&mut a, NamedKey::Delete));
    assert_eq!(a.sidebar.explorer.pinned(), vec![creds.clone()]);
    a.explorer_keys_toggle();
    key(&mut a, NamedKey::ArrowDown);
    key(&mut a, NamedKey::ArrowDown);
    let row = a.sidebar.explorer.keys.unwrap().row;
    assert_eq!(
        a.explorer_items()[row].path,
        creds,
        "the cursor crossed into the pinned root"
    );
    assert!(key(&mut a, NamedKey::Delete));
    assert!(a.sidebar.explorer.pinned().is_empty());
    assert_eq!(
        a.sidebar.explorer.keys.map(|k| k.row),
        Some(1),
        "kept on the list"
    );
}

#[test]
fn remove_unpins_by_path_and_a_missing_path_says_so() {
    let _g = crate::app::theme_test_guard();
    let (here, creds) = two("prm");
    let mut a = app(&here);
    a.nav_command(&format!("explorer add {}/", creds.display()));
    assert_eq!(
        a.sidebar.explorer.pinned(),
        vec![creds.clone()],
        "a trailing / is the same"
    );
    a.nav_command(&format!("explorer remove {}", creds.display()));
    assert!(a.sidebar.explorer.pinned().is_empty());
    a.nav_command("explorer add no/such/folder");
    assert!(said(&a).contains("no such path"), "{}", said(&a));
    a.nav_command("explorer add");
    assert!(said(&a).contains("usage"), "{}", said(&a));
}

/// The tree with a folder pinned under the pane's: a viewer open on a file in it, so the pinned
/// root is opened down to it and its row marked.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn explorer_shot_pinned() {
    use crew_theme::ThemeId;
    let _g = crate::app::theme_test_guard();
    let conf = tree(
        "shotpin",
        &["conf/app.properties", "conf/db.properties", "conf/old/"],
    );
    // A second `conf` elsewhere: the two roots say where each one is.
    let twin = tree("shottwin", &["conf/prod.properties"]).join("conf");
    for id in [ThemeId::TokyoNight, ThemeId::Porcelain] {
        let name = format!("explorer-pinned-{id:?}").to_lowercase();
        let (conf, twin) = (conf.join("conf"), twin.clone());
        let prep = move |a: &mut CrewApp| {
            a.config.show_nav = true;
            a.config.nav_collapsed = false;
            a.config.nav_section = "explorer".into();
            a.sidebar.explorer.pin(&conf);
            a.sidebar.explorer.pin(&twin);
            a.open_view_here(&conf.join("app.properties").to_string_lossy());
            a.input.focused = false;
        };
        let opacity = crate::tubesheer::sheer(id.theme());
        crate::crtglassshot_tests::window_with(&name, id, opacity, &[], &prep);
    }
}

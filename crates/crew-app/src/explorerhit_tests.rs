//! The tree in the app: a click on a drawn row is that row.
use crate::app::CrewApp;
use crate::explorer::tests::tree;

pub(crate) const CELL: (f32, f32) = (8.0, 17.0);

pub(crate) fn app(root: &std::path::Path) -> CrewApp {
    let mut a = CrewApp {
        geo_override: Some((CELL.0, CELL.1, 1280.0, 720.0, 1.0)),
        cwd: root.to_path_buf(),
        ..Default::default()
    };
    a.config.show_nav = true;
    a.config.nav_collapsed = false;
    a.config.nav_section = "explorer".into();
    a.input.focused = false;
    a
}

/// The pixel at the middle of where `word` is drawn in the nav.
pub(crate) fn at(a: &mut CrewApp, word: &str) -> (f32, f32) {
    let scenes = a.build_frame();
    let s = scenes
        .iter()
        .find(|s| s.x < 30.0 && s.h > 60.0)
        .expect("the nav");
    let first = word.chars().next().unwrap();
    let c = s
        .cells
        .iter()
        .filter(|c| c.c == first)
        .find(|c| {
            word.chars().enumerate().all(|(i, ch)| {
                s.cells
                    .iter()
                    .any(|d| d.row == c.row && d.col == c.col + i as u16 && d.c == ch)
            })
        })
        .unwrap_or_else(|| panic!("{word} drawn"));
    (
        s.x + (f32::from(c.col) + 0.5) * CELL.0,
        s.y + (f32::from(c.row) + 0.5) * CELL.1,
    )
}

#[test]
fn a_click_on_a_folder_opens_it_and_on_a_file_views_it() {
    let _g = crate::app::theme_test_guard();
    let root = tree("click", &["docs/guide.md", "notes.txt"]);
    let mut a = app(&root);
    a.cursor = at(&mut a, "docs");
    assert!(a.explorer_press());
    a.cursor = at(&mut a, "guide.md");
    assert!(a.explorer_press());
    assert_eq!(a.panes.len(), 1, "the file opened in a viewer pane");
    assert_eq!(a.sidebar.explorer.picked, Some(root.join("docs/guide.md")));
    // …and a click on the folder again closes it.
    a.cursor = at(&mut a, "docs");
    a.explorer_press();
    assert_eq!(a.sidebar.explorer.items(&root).len(), 3);
}

/// Every drawn row answers for the item drawn on it, scrolled or not.
#[test]
fn every_drawn_row_is_the_item_drawn_on_it() {
    let _g = crate::app::theme_test_guard();
    let names: Vec<String> = (0..60).map(|i| format!("file{i:02}.txt")).collect();
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let root = tree("rows", &refs);
    let mut a = app(&root);
    for scroll in [0usize, 7] {
        a.sidebar.explorer.scroll = scroll;
        for name in ["file10.txt", "file20.txt"] {
            a.cursor = at(&mut a, name);
            let k = a.explorer_item_at_cursor().expect("an item");
            assert_eq!(
                a.sidebar.explorer.items(&root)[k].name,
                name,
                "scroll={scroll}"
            );
        }
    }
    // The wheel moves the list, and only so far.
    a.cursor = at(&mut a, "file20.txt");
    assert!(a.explorer_scroll(-1000));
    let last = a.sidebar.explorer.scroll;
    assert!(last > 0 && last < 61, "{last}");
}

/// Off the Explorer section the tree answers nothing.
#[test]
fn another_section_does_not_answer_for_the_tree() {
    let _g = crate::app::theme_test_guard();
    let root = tree("other", &["a.txt"]);
    let mut a = app(&root);
    a.cursor = at(&mut a, "a.txt");
    a.config.nav_section = "panes".into();
    assert_eq!(a.explorer_item_at_cursor(), None);
}

/// Opening a file moves focus to its viewer, which has no directory of its own: the tree must
/// keep its root rather than swing to crew's, and the next file must reuse that viewer rather
/// than stack a pane (or a tab) per click.
#[test]
fn opening_files_keeps_the_root_and_reuses_one_viewer() {
    let _g = crate::app::theme_test_guard();
    let root = tree("preview", &["one.txt", "two.txt"]);
    let elsewhere = tree("elsewhere", &["unrelated.txt"]);
    let mut a = app(&elsewhere);
    let mut shell = crate::glassworkshot_tests::pane(
        crate::pane::PaneContent::Todo(crate::todopane::TodoPane::new()),
        None,
    );
    shell.dir = Some(root.clone());
    a.panes.push(shell);
    a.focused = 0;
    a.reconcile_grid();
    assert_eq!(a.explorer_root(), root);
    let tabs = a.tabs.list.len();
    a.cursor = at(&mut a, "one.txt");
    assert!(a.explorer_press());
    assert_eq!(a.panes.len(), 2, "one viewer");
    assert_eq!(
        a.explorer_root(),
        root,
        "focus on the viewer keeps the root"
    );
    a.cursor = at(&mut a, "two.txt");
    assert!(a.explorer_press());
    assert_eq!(a.panes.len(), 2, "the viewer is reused, not stacked");
    assert_eq!(a.tabs.list.len(), tabs, "and no tab per click");
    let crate::pane::PaneContent::View(v) = &a.panes[1].content else {
        panic!("a viewer")
    };
    assert_eq!(v.path, root.join("two.txt"));
}

/// The tree is flattened several times a frame (draw, hover, pointer): with a folder of
/// thousands open it must stay cheap. Printed, not asserted — a debug build is no benchmark.
#[test]
#[ignore = "timing; run by hand"]
fn time_items_with_target_deps_open() {
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let repo = repo.canonicalize().unwrap();
    let mut e = crate::explorer::Explorer::default();
    for sub in ["target", "target/debug", "target/debug/deps"] {
        let items = e.items(&repo);
        let it = items
            .iter()
            .find(|i| i.path == repo.join(sub))
            .expect(sub)
            .clone();
        e.toggle(&it);
    }
    let t = std::time::Instant::now();
    let n = e.items(&repo).len();
    let first = t.elapsed();
    let t = std::time::Instant::now();
    for _ in 0..100 {
        e.items(&repo);
    }
    eprintln!(
        "{n} rows: first {first:?}, then {:?} each",
        t.elapsed() / 100
    );
}

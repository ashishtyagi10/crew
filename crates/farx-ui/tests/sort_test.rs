//! Integration tests for sort toggle (asc/desc) and sort persistence.

use farx_core::{Action, SortField, SortOrder};

mod common;
use common::{make_app_in, setup_test_dir};

#[test]
fn test_sort_by_size_changes_tree_order() {
    let dir = setup_test_dir();
    let mut app = make_app_in(dir.path());

    // Default sort: Name ascending
    let names_before: Vec<String> = app
        .left_tree
        .visible_nodes
        .iter()
        .map(|n| n.entry.name.clone())
        .collect();
    // Should start with "..", then dirs first, then alphabetical files
    assert_eq!(names_before[0], "..");

    // Dispatch sort by size
    app.dispatch(Action::SortBySize);

    let names_after: Vec<String> = app
        .left_tree
        .visible_nodes
        .iter()
        .map(|n| n.entry.name.clone())
        .collect();

    // After sort by size: files should be reordered by size
    // "gamma.rs" (4 bytes) < "delta.py" (11) < "alpha.rs" (12) < "beta.txt" (29)
    let files_after: Vec<&String> = names_after
        .iter()
        .filter(|n| *n != ".." && *n != "subdir")
        .collect();
    assert_eq!(
        files_after,
        vec!["gamma.rs", "delta.py", "alpha.rs", "beta.txt"],
        "Files should be sorted by size ascending"
    );
}

#[test]
fn test_sort_toggle_reverses_order() {
    let dir = setup_test_dir();
    let mut app = make_app_in(dir.path());

    // Sort by size ascending first
    app.dispatch(Action::SortBySize);
    assert_eq!(app.left_panel.sort_field, SortField::Size);
    assert_eq!(app.left_panel.sort_order, SortOrder::Ascending);
    assert_eq!(app.left_tree.sort_field, SortField::Size);
    assert_eq!(app.left_tree.sort_order, SortOrder::Ascending);

    // Sort by size again → should toggle to descending
    app.dispatch(Action::SortBySize);
    assert_eq!(app.left_panel.sort_order, SortOrder::Descending);
    assert_eq!(app.left_tree.sort_order, SortOrder::Descending);

    let names: Vec<String> = app
        .left_tree
        .visible_nodes
        .iter()
        .map(|n| n.entry.name.clone())
        .collect();
    let files: Vec<&String> = names
        .iter()
        .filter(|n| *n != ".." && *n != "subdir")
        .collect();
    assert_eq!(
        files,
        vec!["beta.txt", "alpha.rs", "delta.py", "gamma.rs"],
        "Files should be sorted by size descending"
    );
}

#[test]
fn test_sort_by_extension() {
    let dir = setup_test_dir();
    let mut app = make_app_in(dir.path());

    app.dispatch(Action::SortByExtension);

    let names: Vec<String> = app
        .left_tree
        .visible_nodes
        .iter()
        .map(|n| n.entry.name.clone())
        .collect();
    let files: Vec<&String> = names
        .iter()
        .filter(|n| *n != ".." && *n != "subdir")
        .collect();
    // .py < .rs < .txt (alphabetical by extension)
    assert_eq!(files[0], "delta.py");
    assert!(files[1] == "alpha.rs" || files[1] == "gamma.rs"); // both .rs
    assert!(files[2] == "alpha.rs" || files[2] == "gamma.rs");
    assert_eq!(files[3], "beta.txt");
}

#[test]
fn test_sort_persists_across_rebuild() {
    let dir = setup_test_dir();
    let mut app = make_app_in(dir.path());

    app.dispatch(Action::SortBySize);
    app.left_tree.rebuild(); // simulate what happens on refresh

    // Sort settings should still be there
    assert_eq!(app.left_tree.sort_field, SortField::Size);
    assert_eq!(app.left_tree.sort_order, SortOrder::Ascending);

    let names: Vec<String> = app
        .left_tree
        .visible_nodes
        .iter()
        .map(|n| n.entry.name.clone())
        .collect();
    let files: Vec<&String> = names
        .iter()
        .filter(|n| *n != ".." && *n != "subdir")
        .collect();
    assert_eq!(
        files,
        vec!["gamma.rs", "delta.py", "alpha.rs", "beta.txt"],
        "Sort should persist after rebuild"
    );
}

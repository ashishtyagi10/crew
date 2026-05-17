//! Integration tests for "select by mask" dialog and slash-command behavior.

use farx_core::Action;

mod common;
use common::{make_app_in, setup_test_dir};

#[test]
fn test_select_by_mask_dialog_opens() {
    let dir = setup_test_dir();
    let mut app = make_app_in(dir.path());

    // Before: no dialog
    assert!(app.dialog.is_none());

    // Dispatch select by mask
    app.dispatch(Action::SelectByMaskDialog);

    // Dialog should be open
    assert!(app.dialog.is_some());
}

#[test]
fn test_select_by_mask_via_slash_command() {
    let dir = setup_test_dir();
    let mut app = make_app_in(dir.path());

    // No files selected initially
    assert!(app.left_tree.selected.is_empty());

    // Type "/select *.rs" into command line and execute
    app.command_line.input = "/select *.rs".to_string();
    app.dispatch(Action::CommandLineExecute);

    // Should have selected the .rs files
    let selected_names: Vec<String> = app
        .left_tree
        .selected
        .iter()
        .filter_map(|&idx| app.left_tree.visible_nodes.get(idx))
        .map(|n| n.entry.name.clone())
        .collect();
    assert_eq!(selected_names.len(), 2, "Should select 2 .rs files");
    assert!(selected_names.contains(&"alpha.rs".to_string()));
    assert!(selected_names.contains(&"gamma.rs".to_string()));
}

#[test]
fn test_deselect_by_mask_via_slash_command() {
    let dir = setup_test_dir();
    let mut app = make_app_in(dir.path());

    // First select all
    app.dispatch(Action::SelectAll);
    let initial_count = app.left_tree.selected.len();
    assert!(initial_count > 0);

    // Deselect .rs files
    app.command_line.input = "/deselect *.rs".to_string();
    app.dispatch(Action::CommandLineExecute);

    // .rs files should no longer be selected
    let selected_names: Vec<String> = app
        .left_tree
        .selected
        .iter()
        .filter_map(|&idx| app.left_tree.visible_nodes.get(idx))
        .map(|n| n.entry.name.clone())
        .collect();
    assert!(!selected_names.contains(&"alpha.rs".to_string()));
    assert!(!selected_names.contains(&"gamma.rs".to_string()));
    // But .txt and .py should still be selected
    assert!(selected_names.contains(&"beta.txt".to_string()));
    assert!(selected_names.contains(&"delta.py".to_string()));
}

#[test]
fn test_select_wildcard_star() {
    let dir = setup_test_dir();
    let mut app = make_app_in(dir.path());

    // Select everything with *
    app.command_line.input = "/select *".to_string();
    app.dispatch(Action::CommandLineExecute);

    // All non-".." entries should be selected (files + subdir)
    let count = app.left_tree.selected.len();
    assert!(
        count >= 4,
        "* should select at least 4 items, got {}",
        count
    );
}

#[test]
fn test_select_question_mark_wildcard() {
    let dir = setup_test_dir();
    let mut app = make_app_in(dir.path());

    // Create files with specific length names
    std::fs::write(dir.path().join("a.rs"), "x").unwrap();
    std::fs::write(dir.path().join("bb.rs"), "x").unwrap();
    app.left_tree.rebuild();

    app.command_line.input = "/select ?.rs".to_string();
    app.dispatch(Action::CommandLineExecute);

    let selected_names: Vec<String> = app
        .left_tree
        .selected
        .iter()
        .filter_map(|&idx| app.left_tree.visible_nodes.get(idx))
        .map(|n| n.entry.name.clone())
        .collect();
    assert!(
        selected_names.contains(&"a.rs".to_string()),
        "?.rs should match a.rs"
    );
    assert!(
        !selected_names.contains(&"bb.rs".to_string()),
        "?.rs should NOT match bb.rs"
    );
}

//! Integration tests for slash-command sorting and selection interactions.

use farx_core::{Action, SortField, SortOrder};

mod common;
use common::{make_app_in, setup_test_dir};

#[test]
fn test_sort_slash_command() {
    let dir = setup_test_dir();
    let mut app = make_app_in(dir.path());

    app.command_line.input = "/sort size".to_string();
    app.dispatch(Action::CommandLineExecute);

    assert_eq!(app.left_panel.sort_field, SortField::Size);
    assert_eq!(app.left_tree.sort_field, SortField::Size);
}

#[test]
fn test_sort_slash_command_toggle() {
    let dir = setup_test_dir();
    let mut app = make_app_in(dir.path());

    // First call: set to size ascending
    app.command_line.input = "/sort size".to_string();
    app.dispatch(Action::CommandLineExecute);
    assert_eq!(app.left_panel.sort_order, SortOrder::Ascending);

    // Second call: toggle to descending
    app.command_line.input = "/sort size".to_string();
    app.dispatch(Action::CommandLineExecute);
    assert_eq!(app.left_panel.sort_order, SortOrder::Descending);
}

#[test]
fn test_sort_clears_stale_selection() {
    let dir = setup_test_dir();
    let mut app = make_app_in(dir.path());

    // Select some files by mask
    app.command_line.input = "/select *.rs".to_string();
    app.dispatch(Action::CommandLineExecute);
    assert!(!app.left_tree.selected.is_empty(), "Should have selections");

    // Now sort by size — selection indices refer to the old order
    app.dispatch(Action::SortBySize);

    // Verify that if any selection indices remain, they still point to valid nodes
    for &idx in &app.left_tree.selected {
        assert!(
            idx < app.left_tree.visible_nodes.len(),
            "Stale index {} out of bounds (len={})",
            idx,
            app.left_tree.visible_nodes.len()
        );
    }
}

#[test]
fn test_select_then_sort_then_select_works() {
    let dir = setup_test_dir();
    let mut app = make_app_in(dir.path());

    // Select .rs files
    app.command_line.input = "/select *.rs".to_string();
    app.dispatch(Action::CommandLineExecute);

    // Sort by size
    app.dispatch(Action::SortBySize);

    // Deselect all
    app.dispatch(Action::DeselectAll);
    assert!(app.left_tree.selected.is_empty());

    // Select .txt files — should work cleanly after sort
    app.command_line.input = "/select *.txt".to_string();
    app.dispatch(Action::CommandLineExecute);

    let selected_names: Vec<String> = app
        .left_tree
        .selected
        .iter()
        .filter_map(|&idx| app.left_tree.visible_nodes.get(idx))
        .map(|n| n.entry.name.clone())
        .collect();
    assert_eq!(selected_names.len(), 1);
    assert!(selected_names.contains(&"beta.txt".to_string()));
}

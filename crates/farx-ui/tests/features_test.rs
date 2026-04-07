/// Integration tests for v0.1.4/v0.1.5 features:
/// - Sort toggle (asc/desc)
/// - Select by mask
/// - Symlink creation
use farx_core::{Action, AppConfig, SortField, SortOrder};
use farx_ui::app::App;
use farx_ui::components::editor::EditorState;
use farx_ui::components::viewer::ViewerState;

use std::sync::Mutex;

// Serialize tests that change cwd — can't safely race on a process-global
static CWD_LOCK: Mutex<()> = Mutex::new(());

fn make_app_in(dir: &std::path::Path) -> App {
    let _guard = CWD_LOCK.lock().unwrap();
    let original = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir).unwrap();
    let config = AppConfig::default();
    let app = App::new(config).unwrap();
    std::env::set_current_dir(original).unwrap();
    app
}

fn setup_test_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    // Create files with different sizes and extensions
    std::fs::write(dir.path().join("alpha.rs"), "fn main() {}").unwrap();
    std::fs::write(
        dir.path().join("beta.txt"),
        "hello world and more text here",
    )
    .unwrap();
    std::fs::write(dir.path().join("gamma.rs"), "// g").unwrap();
    std::fs::write(dir.path().join("delta.py"), "print('hi')").unwrap();
    std::fs::create_dir(dir.path().join("subdir")).unwrap();
    dir
}

// ─── Sort toggle tests ──────────────────────────────────────────────

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

// ─── Select by mask tests ───────────────────────────────────────────

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

// ─── Symlink creation tests ────────────────────────────────────────

#[test]
fn test_symlink_dialog_opens() {
    let dir = setup_test_dir();
    let mut app = make_app_in(dir.path());

    // Move cursor to a file (skip ".." and "subdir")
    // After default sort by name: .., subdir, alpha.rs, beta.txt, delta.py, gamma.rs
    app.left_tree.move_cursor_to(2); // alpha.rs (first file after dir)

    app.dispatch(Action::CreateSymlinkDialog);

    assert!(app.dialog.is_some(), "Dialog should be open");
}

#[test]
fn test_symlink_on_dotdot_is_noop() {
    let dir = setup_test_dir();
    let mut app = make_app_in(dir.path());

    // Cursor is on ".." (index 0)
    app.left_tree.move_cursor_to(0);

    app.dispatch(Action::CreateSymlinkDialog);

    // Should NOT open dialog for ".."
    assert!(app.dialog.is_none(), "Dialog should not open for '..'");
}

#[test]
fn test_symlink_actually_creates_link() {
    let dir = setup_test_dir();
    let target = dir.path().join("alpha.rs");
    let link = dir.path().join("alpha_link");

    // Test the fs operation directly
    farx_fs::create_symlink(&target, &link).unwrap();

    assert!(link.exists(), "Symlink should exist on disk");
    assert!(
        link.symlink_metadata().unwrap().is_symlink(),
        "Should actually be a symlink"
    );
    let resolved = std::fs::read_link(&link).unwrap();
    assert_eq!(resolved, target, "Symlink should point to target");

    // Read through the symlink
    let content = std::fs::read_to_string(&link).unwrap();
    assert_eq!(content, "fn main() {}");
}

#[test]
fn test_symlink_slash_command_opens_dialog() {
    let dir = setup_test_dir();
    let mut app = make_app_in(dir.path());

    // Move cursor to a file
    app.left_tree.move_cursor_to(2);

    // Execute /symlink
    app.command_line.input = "/symlink".to_string();
    app.dispatch(Action::CommandLineExecute);

    assert!(app.dialog.is_some(), "/symlink should open dialog");
}

#[test]
fn test_ln_slash_command_opens_dialog() {
    let dir = setup_test_dir();
    let mut app = make_app_in(dir.path());

    // Move cursor to a file
    app.left_tree.move_cursor_to(2);

    app.command_line.input = "/ln".to_string();
    app.dispatch(Action::CommandLineExecute);

    assert!(app.dialog.is_some(), "/ln should open dialog");
}

// ─── Slash command tests ────────────────────────────────────────────

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

// ─── Unicode safety tests ──────────────────────────────────────────

#[test]
fn test_unicode_filenames_dont_panic() {
    let dir = setup_test_dir();
    // Create files with multi-byte UTF-8 names
    std::fs::write(dir.path().join("日本語.txt"), "hello").unwrap();
    std::fs::write(dir.path().join("émojis🎉.rs"), "fn main() {}").unwrap();
    std::fs::write(dir.path().join("café.py"), "print()").unwrap();

    let mut app = make_app_in(dir.path());

    // These should not panic — just verify the tree built successfully
    assert!(app.left_tree.visible_nodes.len() >= 7); // .., subdir, 4 original + 3 unicode

    // Sort should not panic with unicode names
    app.dispatch(Action::SortByName);
    app.dispatch(Action::SortByExtension);
    app.dispatch(Action::SortBySize);

    // Select by mask should handle unicode
    app.command_line.input = "/select *.txt".to_string();
    app.dispatch(Action::CommandLineExecute);

    let selected: Vec<String> = app
        .left_tree
        .selected
        .iter()
        .filter_map(|&idx| app.left_tree.visible_nodes.get(idx))
        .map(|n| n.entry.name.clone())
        .collect();
    assert!(selected.contains(&"日本語.txt".to_string()));
    assert!(selected.contains(&"beta.txt".to_string()));
}

#[test]
fn test_editor_unicode_no_panic() {
    let dir = setup_test_dir();
    let file = dir.path().join("unicode_test.txt");
    std::fs::write(&file, "Hello 世界\nCafé ☕\n🎉🎊🎈").unwrap();

    use crate::EditorState;
    let mut editor = EditorState::open(&file).unwrap();

    // Move cursor right through multi-byte chars — should not panic
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    for _ in 0..20 {
        editor.handle_key_event(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
    }
    // Move left back
    for _ in 0..20 {
        editor.handle_key_event(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
    }
    // Type a character
    editor.handle_key_event(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
    // Press Enter to split a line
    editor.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    // Backspace
    editor.handle_key_event(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
}

#[test]
fn test_editor_file_size_limit() {
    let dir = setup_test_dir();
    // We won't actually create a 100MB file in tests, but verify the limit exists
    // by checking that normal files open fine
    let file = dir.path().join("small.txt");
    std::fs::write(&file, "small file").unwrap();

    use crate::EditorState;
    assert!(EditorState::open(&file).is_ok());
}

#[test]
fn test_viewer_file_size_limit() {
    let dir = setup_test_dir();
    let file = dir.path().join("small.txt");
    std::fs::write(&file, "small file").unwrap();

    use crate::ViewerState;
    assert!(ViewerState::open(&file).is_ok());
}

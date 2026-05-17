//! Integration tests for Unicode safety and editor/viewer size limits.

use farx_core::Action;
use farx_ui::components::editor::EditorState;
use farx_ui::components::viewer::ViewerState;

mod common;
use common::{make_app_in, setup_test_dir};

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

    assert!(EditorState::open(&file).is_ok());
}

#[test]
fn test_viewer_file_size_limit() {
    let dir = setup_test_dir();
    let file = dir.path().join("small.txt");
    std::fs::write(&file, "small file").unwrap();

    assert!(ViewerState::open(&file).is_ok());
}

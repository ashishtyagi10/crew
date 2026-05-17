//! Integration tests for symlink creation: dialog, slash-commands, fs op.

use farx_core::Action;

mod common;
use common::{make_app_in, setup_test_dir};

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

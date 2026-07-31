//! F3/F4 routing: `view_selected`/`edit_selected` must yield the action that
//! keeps the file inside crew, and must still ignore a directory selection —
//! the one behaviour this task's split was required to preserve exactly.
use super::{edit_selected, view_selected, FarAction, FarPane};

/// A FarPane rooted at a unique temp dir containing one subdirectory and one
/// file, with the file selected in the left (active) panel. `key` keeps each
/// test isolated so the parallel runner can't race on a shared path.
fn fixture(key: &str) -> (std::path::PathBuf, FarPane) {
    let base = std::env::temp_dir().join(format!("crew_far_keys_{key}"));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(base.join("sub")).unwrap();
    std::fs::write(base.join("f.txt"), b"x").unwrap();
    let mut pane = FarPane::new(base.clone());
    select(&mut pane, "f.txt");
    (base, pane)
}

fn select(p: &mut FarPane, name: &str) {
    let i = p.left.entries.iter().position(|e| e.name == name).unwrap();
    p.left.sel = i;
}

#[test]
fn f3_views_the_selected_local_file() {
    let (base, mut p) = fixture("view");
    let Some(FarAction::View(path)) = view_selected(&mut p) else {
        panic!("F3 on a local file must yield FarAction::View");
    };
    assert_eq!(path, base.join("f.txt"));
}

#[test]
fn f4_edits_the_selected_local_file() {
    let (base, mut p) = fixture("edit");
    let Some(FarAction::Edit(path)) = edit_selected(&mut p) else {
        panic!("F4 on a local file must yield FarAction::Edit");
    };
    assert_eq!(path, base.join("f.txt"));
}

#[test]
fn f3_and_f4_ignore_a_directory_selection() {
    let (_, mut p) = fixture("dirs");
    select(&mut p, "sub");
    assert!(
        view_selected(&mut p).is_none(),
        "F3 on a directory is a no-op"
    );
    assert!(
        edit_selected(&mut p).is_none(),
        "F4 on a directory is a no-op"
    );
}

#[test]
fn f3_and_f4_ignore_the_parent_row() {
    let (_, mut p) = fixture("parent");
    p.left.sel = 0; // the ".." row
    assert!(view_selected(&mut p).is_none(), "F3 on '..' is a no-op");
    assert!(edit_selected(&mut p).is_none(), "F4 on '..' is a no-op");
}

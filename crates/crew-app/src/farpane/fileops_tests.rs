use super::*;
use crate::farpane::keys::FarAction;
use crate::farpane::location::Location;
use crate::farpane::FarPane;

/// A FarPane rooted at a unique temp dir containing one subdirectory and one
/// file. `key` keeps each test isolated so the parallel runner can't race on a
/// shared path.
fn fixture(key: &str) -> (std::path::PathBuf, FarPane) {
    let base = std::env::temp_dir().join(format!("crew_far_ops_{key}"));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(base.join("sub")).unwrap();
    std::fs::write(base.join("f.txt"), b"x").unwrap();
    let pane = FarPane::new(base.clone());
    (base, pane)
}

/// Select the named entry in the left (active) panel.
fn select(p: &mut FarPane, name: &str) {
    let i = p.left.entries.iter().position(|e| e.name == name).unwrap();
    p.left.sel = i;
}

#[test]
fn f5_copies_selected_file_to_the_other_panel() {
    let (base, mut p) = fixture("copy");
    p.right.loc = Location::local(&base.join("sub"));
    p.right.reload();
    select(&mut p, "f.txt");
    let action = copy(&mut p);
    assert!(matches!(action, FarAction::Status(_)));
    assert!(
        base.join("sub/f.txt").exists(),
        "file copied into other panel"
    );
    assert!(base.join("f.txt").exists(), "original is left in place");
}

#[test]
fn f5_copy_will_not_clobber_an_existing_file() {
    let (base, mut p) = fixture("copy_clobber");
    std::fs::create_dir_all(base.join("sub")).unwrap();
    std::fs::write(base.join("sub/f.txt"), b"original").unwrap();
    p.right.loc = Location::local(&base.join("sub"));
    p.right.reload();
    select(&mut p, "f.txt");
    copy(&mut p);
    // the existing destination file is untouched
    assert_eq!(std::fs::read(base.join("sub/f.txt")).unwrap(), b"original");
}

#[test]
fn f6_moves_selected_file_to_the_other_panel() {
    let (base, mut p) = fixture("move");
    p.right.loc = Location::local(&base.join("sub"));
    p.right.reload();
    select(&mut p, "f.txt");
    rename_move(&mut p);
    assert!(
        base.join("sub/f.txt").exists(),
        "file moved into other panel"
    );
    assert!(
        !base.join("f.txt").exists(),
        "original is gone after a move"
    );
}

#[test]
fn f7_make_dir_creates_a_folder() {
    let (base, mut p) = fixture("mkdir");
    let action = make_dir(&mut p, "newdir");
    assert!(matches!(action, FarAction::Status(_)));
    assert!(base.join("newdir").is_dir());
    // the active panel re-reads so the new folder is listed
    assert!(p
        .left
        .entries
        .iter()
        .any(|e| e.name == "newdir" && e.is_dir));
}

#[test]
fn f8_delete_refuses_the_parent_entry() {
    let (base, mut p) = fixture("del_parent");
    p.left.sel = 0; // the ".." row
    delete(&mut p);
    // nothing was removed
    assert!(base.join("f.txt").exists());
    assert!(base.join("sub").exists());
}

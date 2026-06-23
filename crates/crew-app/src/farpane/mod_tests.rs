use super::keys::{activate, ascend, move_sel};
use super::{FarPane, Side};

/// A FarPane rooted at a unique temp dir containing one subdirectory and one
/// file. `key` keeps each test isolated so the parallel runner can't race on a
/// shared path.
fn fixture(key: &str) -> (std::path::PathBuf, FarPane) {
    let base = std::env::temp_dir().join(format!("crew_far_mod_{key}"));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(base.join("sub")).unwrap();
    std::fs::write(base.join("f.txt"), b"x").unwrap();
    let pane = FarPane::new(base.clone());
    (base, pane)
}

#[test]
fn starts_active_left_on_given_dir() {
    let (base, p) = fixture("start");
    assert!(matches!(p.active, Side::Left));
    assert_eq!(p.left.cwd, base);
    assert_eq!(p.right.cwd, base);
    // ".." + "sub/" + "f.txt"
    assert_eq!(p.left.entries.len(), 3);
}

#[test]
fn tab_switches_active_panel() {
    let (_b, mut p) = fixture("tab");
    p.active = Side::Right;
    move_sel(&mut p, 1); // moves the RIGHT panel, not the left
    assert_eq!(p.right.sel, 1);
    assert_eq!(p.left.sel, 0);
}

#[test]
fn enter_descends_into_dir_and_back() {
    let (base, mut p) = fixture("descend");
    // entries[1] is "sub/" (dirs sort before files, after "..")
    p.left.sel = 1;
    activate(&mut p);
    assert_eq!(p.left.cwd, base.join("sub"));
    // the child dir has a ".." entry to climb back out
    assert!(p.left.entries.iter().any(|e| e.is_parent));
    ascend(&mut p);
    assert_eq!(p.left.cwd, base);
}

#[test]
fn move_sel_clamps_to_bounds() {
    let (_b, mut p) = fixture("enter");
    move_sel(&mut p, -5);
    assert_eq!(p.left.sel, 0);
    move_sel(&mut p, 99);
    assert_eq!(p.left.sel, p.left.entries.len() - 1);
}

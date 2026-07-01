use super::keys::{activate, ascend, copy, delete, make_dir, move_sel, rename_move, run_cmdline};
use super::{FarAction, FarPane, Side};

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
fn command_line_runs_in_active_panel_dir() {
    let (base, mut p) = fixture("cmdline");
    // Type a command on the right panel (which points at a subdir).
    p.active = Side::Right;
    p.right.cwd = base.join("sub");
    p.cmdline = "ls -la".into();
    match run_cmdline(&mut p) {
        FarAction::Run { cmd, cwd } => {
            assert_eq!(cmd, "ls -la");
            assert_eq!(cwd, base.join("sub")); // active panel's dir, not left's
        }
        _ => panic!("expected FarAction::Run"),
    }
    // Running consumes the command line.
    assert!(p.cmdline.is_empty());
}

#[test]
fn whitespace_command_line_does_not_run() {
    let (_b, mut p) = fixture("blankcmd");
    p.cmdline = "   ".into();
    assert!(matches!(run_cmdline(&mut p), FarAction::Status(_)));
}

#[test]
fn move_sel_clamps_to_bounds() {
    let (_b, mut p) = fixture("enter");
    move_sel(&mut p, -5);
    assert_eq!(p.left.sel, 0);
    move_sel(&mut p, 99);
    assert_eq!(p.left.sel, p.left.entries.len() - 1);
}

#[test]
fn scroll_moves_active_cursor_clamped() {
    let (_b, mut p) = fixture("scroll");
    p.scroll(-99); // wheel down → toward bottom
    assert_eq!(p.left.sel, p.left.entries.len() - 1);
    p.scroll(99); // wheel up → toward top
    assert_eq!(p.left.sel, 0);
}

/// Select the named entry in the left (active) panel.
fn select(p: &mut FarPane, name: &str) {
    let i = p.left.entries.iter().position(|e| e.name == name).unwrap();
    p.left.sel = i;
}

#[test]
fn f5_copies_selected_file_to_the_other_panel() {
    let (base, mut p) = fixture("copy");
    p.right.cwd = base.join("sub");
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
    p.right.cwd = base.join("sub");
    p.right.reload();
    select(&mut p, "f.txt");
    copy(&mut p);
    // the existing destination file is untouched
    assert_eq!(std::fs::read(base.join("sub/f.txt")).unwrap(), b"original");
}

#[test]
fn f6_moves_selected_file_to_the_other_panel() {
    let (base, mut p) = fixture("move");
    p.right.cwd = base.join("sub");
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

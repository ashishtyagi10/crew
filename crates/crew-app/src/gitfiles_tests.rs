use super::*;

const OUT: &str = "## main...origin/main [ahead 1]
 M crates/app/src/main.rs
M  docs/guide.md
A  new.rs
?? scratch/
 D gone.rs
UU merge.rs
R  old.rs -> renamed.rs
";

#[test]
fn porcelain_lines_become_marks_under_the_root() {
    let root = Path::new("/r");
    let f = parse(OUT, root);
    let m = |p: &str| f.mark(&root.join(p));
    assert_eq!(
        m("crates/app/src/main.rs"),
        Some(Mark::Modified),
        "worktree change"
    );
    assert_eq!(m("docs/guide.md"), Some(Mark::Modified), "staged change");
    assert_eq!(m("new.rs"), Some(Mark::Added));
    assert_eq!(m("gone.rs"), Some(Mark::Deleted));
    assert_eq!(m("merge.rs"), Some(Mark::Conflict));
    assert_eq!(
        m("renamed.rs"),
        Some(Mark::Modified),
        "the new name of a rename"
    );
    assert_eq!(m("old.rs"), None);
    assert_eq!(m("README.md"), None);
}

/// Git lists a new folder once; every file in it is new.
#[test]
fn a_file_in_an_untracked_folder_is_untracked() {
    let root = Path::new("/r");
    let f = parse(OUT, root);
    assert_eq!(f.mark(&root.join("scratch/a/b.txt")), Some(Mark::Untracked));
    assert_eq!(f.mark(&root.join("scratch")), Some(Mark::Untracked));
}

#[test]
fn a_folder_holds_changes_when_anything_under_it_is_marked() {
    let root = Path::new("/r");
    let f = parse(OUT, root);
    assert!(f.holds_changes(&root.join("crates")));
    assert!(f.holds_changes(&root.join("crates/app/src")));
    assert!(
        !f.holds_changes(&root.join("crates/app/src/main.rs")),
        "a file holds nothing"
    );
    assert!(!f.holds_changes(&root.join("assets")));
}

/// The real thing: a repo with one changed file, asked from a folder below its root.
#[test]
fn query_from_a_subfolder_marks_paths_from_the_root() {
    let dir = std::env::temp_dir().join(format!("crew_gitfiles_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("sub")).unwrap();
    let git = |args: &[&str]| {
        std::process::Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args(args)
            .output()
            .expect("git")
    };
    if !git(&["init", "-q"]).status.success() {
        return; // no git on this machine
    }
    std::fs::write(dir.join("sub/new.txt"), "x").unwrap();
    let (info, files) = query(&dir.join("sub")).expect("a repo");
    assert_eq!(info.changed, 1);
    let at = dir.join("sub/new.txt").canonicalize().unwrap();
    let found = files
        .mark(&at)
        .or_else(|| files.mark(&dir.join("sub/new.txt")));
    assert_eq!(found, Some(Mark::Untracked));
    let _ = std::fs::remove_dir_all(&dir);
}

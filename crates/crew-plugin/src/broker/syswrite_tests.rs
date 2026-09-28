use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::write_in;

/// A fresh directory with a `proj` inside it to stand in as the working
/// directory, so "outside" has somewhere to be that is still the test's own.
fn tree() -> (PathBuf, PathBuf) {
    static N: AtomicUsize = AtomicUsize::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let top = std::env::temp_dir().join(format!("crew-syswrite-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&top);
    std::fs::create_dir_all(top.join("proj")).unwrap();
    (top.clone(), top.join("proj"))
}

#[test]
fn a_new_file_in_a_new_directory_is_written_with_its_directories() {
    let (top, cwd) = tree();
    let out = write_in(&cwd, "new/dir/a.rs", "fn a() {}\n").unwrap();
    assert_eq!(out, "wrote 10 bytes to new/dir/a.rs");
    let wrote = std::fs::read_to_string(cwd.join("new").join("dir").join("a.rs")).unwrap();
    assert_eq!(wrote, "fn a() {}\n");
    let _ = std::fs::remove_dir_all(&top);
}

/// A `..` among the names that do not exist yet is folded before anything is
/// made, so the file lands where the path says and no stray directory is left.
#[test]
fn a_dot_dot_among_the_missing_names_lands_where_it_says() {
    let (top, cwd) = tree();
    write_in(&cwd, "new/../made/a.rs", "x").unwrap();
    assert!(cwd.join("made").join("a.rs").is_file());
    assert!(!cwd.join("new").exists(), "a stray directory was made");
    let _ = std::fs::remove_dir_all(&top);
}

#[test]
fn no_directory_is_made_outside_the_working_directory() {
    let (top, cwd) = tree();
    let abs = top.join("elsewhere").join("new").join("a.rs");
    let rel = ["..", "escape", "a.rs"].iter().collect::<PathBuf>();
    let sly = ["new", "..", "..", "sly", "a.rs"]
        .iter()
        .collect::<PathBuf>();
    for path in [abs, rel, sly] {
        let path = path.to_str().unwrap();
        let e = write_in(&cwd, path, "x").unwrap_err();
        assert!(e.starts_with(&format!("write {path}: ")), "{e}");
        assert!(e.contains("only inside the working directory"), "{e}");
        assert!(e.contains("sys:run mkdir -p"), "{e}");
    }
    for made in ["elsewhere", "escape", "sly"] {
        assert!(!top.join(made).exists(), "{made} was made");
    }
    assert!(!cwd.join("new").exists());
    let _ = std::fs::remove_dir_all(&top);
}

/// Not a sandbox: outside the project a directory that is already there is
/// written into as it always was. Only MAKING one is kept inside.
#[test]
fn outside_the_working_directory_a_directory_that_exists_is_written_as_before() {
    let (top, cwd) = tree();
    let p = top.join("a.txt");
    write_in(&cwd, p.to_str().unwrap(), "hi").unwrap();
    assert_eq!(std::fs::read_to_string(&p).unwrap(), "hi");
    let _ = std::fs::remove_dir_all(&top);
}

/// Inside by name is not inside: a link in the project that points out of it
/// is followed before the question is asked.
#[cfg(unix)]
#[test]
fn a_link_out_of_the_project_is_outside_it() {
    let (top, cwd) = tree();
    std::fs::create_dir_all(top.join("out")).unwrap();
    std::os::unix::fs::symlink(top.join("out"), cwd.join("link")).unwrap();
    let e = write_in(&cwd, "link/new/a.rs", "x").unwrap_err();
    assert!(e.contains("only inside the working directory"), "{e}");
    assert!(!top.join("out").join("new").exists());
    let _ = std::fs::remove_dir_all(&top);
}

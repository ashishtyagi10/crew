use super::expand_path;
use std::path::{Path, PathBuf};

#[test]
fn keeps_absolute_and_joins_relative() {
    let base = Path::new("/work");
    assert_eq!(expand_path(base, "/etc/hosts"), Path::new("/etc/hosts"));
    assert_eq!(expand_path(base, "src/main.rs"), base.join("src/main.rs"));
}

#[test]
fn expands_tilde_and_env() {
    // `with_home` holds the crate-wide `$HOME` lock and restores the
    // prior value — a bare `set_var` here raced every other `$HOME`
    // reader and leaked `/home/u` into the rest of the test run.
    crate::envlock::with_home(Path::new("/home/u"), || {
        assert_eq!(expand_path(Path::new("/x"), "~"), PathBuf::from("/home/u"));
        assert_eq!(
            expand_path(Path::new("/x"), "~/notes.md"),
            PathBuf::from("/home/u/notes.md")
        );
    });
    std::env::set_var("CREW_PE_DIR", "/data");
    // `$VAR` expands, then is treated as the (absolute) path.
    assert_eq!(
        expand_path(Path::new("/x"), "$CREW_PE_DIR/f.txt"),
        PathBuf::from("/data/f.txt")
    );
}

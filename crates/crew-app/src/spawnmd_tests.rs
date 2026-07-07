use super::*;

#[test]
fn resolve_md_path_keeps_absolute_paths() {
    let cwd = Path::new("/some/cwd");
    assert_eq!(
        resolve_md_path(cwd, "/etc/hosts"),
        PathBuf::from("/etc/hosts")
    );
}

#[test]
fn resolve_md_path_joins_relative_paths_onto_cwd() {
    let cwd = Path::new("/some/cwd");
    assert_eq!(
        resolve_md_path(cwd, "README.md"),
        PathBuf::from("/some/cwd/README.md")
    );
}

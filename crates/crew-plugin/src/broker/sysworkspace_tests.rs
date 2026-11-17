use super::*;

fn tree() -> (PathBuf, PathBuf) {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let top = std::env::temp_dir().join(format!("crew-workspace-{}-{n}", std::process::id()));
    let root = top.join("project");
    let _ = std::fs::remove_dir_all(&top);
    std::fs::create_dir_all(&root).unwrap();
    (top.canonicalize().unwrap(), root.canonicalize().unwrap())
}

#[test]
fn accepts_paths_inside_and_rejects_parent_and_absolute_escapes() {
    let (top, root) = tree();
    assert_eq!(
        inside(&root, Path::new("src/new.rs")),
        Some(root.join("src/new.rs"))
    );
    assert_eq!(inside(&root, Path::new("../outside")), None);
    assert_eq!(inside(&root, &top.join("outside")), None);
    let _ = std::fs::remove_dir_all(top);
}

#[cfg(unix)]
#[test]
fn resolves_symlinks_before_accepting_a_path() {
    let (top, root) = tree();
    let outside = top.join("outside");
    std::fs::create_dir_all(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, root.join("link")).unwrap();
    std::fs::create_dir_all(root.join("inside")).unwrap();
    std::os::unix::fs::symlink(root.join("inside"), root.join("local")).unwrap();

    assert_eq!(inside(&root, Path::new("link/secret")), None);
    assert_eq!(
        inside(&root, Path::new("local/file")),
        Some(root.join("inside/file"))
    );
    assert!(discovered_in_workspace(&root, &root.join("inside/file")));
    let _ = std::fs::remove_dir_all(top);
}

#[cfg(unix)]
#[test]
fn discovered_symlink_files_escaping_the_workspace_are_skipped() {
    let (top, root) = tree();
    let outside = top.join("outside.txt");
    std::fs::write(&outside, "secret").unwrap();
    let link = root.join("secret.txt");
    std::os::unix::fs::symlink(outside, &link).unwrap();
    assert!(!discovered_in_workspace(&root, &link));
    let _ = std::fs::remove_dir_all(top);
}

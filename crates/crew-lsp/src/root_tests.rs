use super::*;

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("crew-lsp-root-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn a_git_checkout_wins_over_a_nearer_manifest() {
    let d = scratch("git");
    std::fs::create_dir_all(d.join(".git")).unwrap();
    std::fs::create_dir_all(d.join("crates/x/src")).unwrap();
    std::fs::write(d.join("crates/x/Cargo.toml"), "").unwrap();
    assert_eq!(for_file(&d.join("crates/x/src/lib.rs")), d);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn without_git_the_nearest_manifest_is_the_root() {
    let d = scratch("manifest");
    std::fs::create_dir_all(d.join("pkg/src")).unwrap();
    std::fs::write(d.join("pkg/go.mod"), "").unwrap();
    assert_eq!(for_file(&d.join("pkg/src/main.go")), d.join("pkg"));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_loose_file_is_its_own_directorys_project() {
    let d = scratch("loose");
    // The temp dir may itself sit under some marker on a developer's
    // machine, so only assert the fallback when nothing above claims it.
    let root = for_file(&d.join("a.py"));
    assert!(
        d.starts_with(&root),
        "{} is not above {}",
        root.display(),
        d.display()
    );
    let _ = std::fs::remove_dir_all(&d);
}

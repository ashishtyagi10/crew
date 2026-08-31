use super::*;

/// Build a throwaway tree under the OS temp dir; unique per test run.
fn fixture(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("crew-fileindex-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::create_dir_all(dir.join("target/debug")).unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();
    std::fs::write(dir.join("README.md"), "hi").unwrap();
    std::fs::write(dir.join("src/main.rs"), "fn main() {}").unwrap();
    std::fs::write(dir.join("src/.hidden"), "x").unwrap();
    std::fs::write(dir.join("target/debug/junk"), "x").unwrap();
    std::fs::write(dir.join(".git/config"), "x").unwrap();
    dir
}

#[test]
fn scan_lists_files_relative_and_sorted() {
    let dir = fixture("basic");
    let files = scan(&dir);
    assert_eq!(
        files,
        vec!["README.md".to_string(), "src/main.rs".to_string()]
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn scan_skips_hidden_and_build_dirs() {
    let dir = fixture("skips");
    let files = scan(&dir);
    assert!(!files
        .iter()
        .any(|f| f.contains(".git") || f.contains("target") || f.contains(".hidden")));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn scan_of_missing_dir_is_empty() {
    assert!(scan(Path::new("/nonexistent/definitely-not-here")).is_empty());
}

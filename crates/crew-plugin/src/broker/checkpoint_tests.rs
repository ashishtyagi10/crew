use std::path::PathBuf;
use std::process::Command;

use super::*;

/// A throwaway git repo with one committed file, isolated per test.
fn temp_repo(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "crew-ckpt-test-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0),
    ));
    std::fs::create_dir_all(&dir).unwrap();
    for args in [
        &["init", "-q"][..],
        &["config", "user.email", "t@t"],
        &["config", "user.name", "t"],
    ] {
        assert!(Command::new("git")
            .args(args)
            .current_dir(&dir)
            .status()
            .unwrap()
            .success());
    }
    std::fs::write(dir.join("a.txt"), "one").unwrap();
    for args in [&["add", "-A"][..], &["commit", "-q", "-m", "init"]] {
        assert!(Command::new("git")
            .args(args)
            .current_dir(&dir)
            .status()
            .unwrap()
            .success());
    }
    dir
}

#[test]
fn snapshot_lists_with_label_and_restores_edits() {
    let dir = temp_repo("roundtrip");
    std::fs::write(dir.join("a.txt"), "two").unwrap();
    snapshot(&dir, "before the agent runs").unwrap();
    let items = list(&dir).unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].1, "before the agent runs");

    std::fs::write(dir.join("a.txt"), "three").unwrap();
    restore(&dir, &items[0].0).unwrap();
    assert_eq!(std::fs::read_to_string(dir.join("a.txt")).unwrap(), "two");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn snapshot_captures_untracked_files_and_restore_brings_them_back() {
    let dir = temp_repo("untracked");
    std::fs::write(dir.join("b.txt"), "new file").unwrap();
    snapshot(&dir, "with b").unwrap();
    std::fs::remove_file(dir.join("b.txt")).unwrap();

    let items = list(&dir).unwrap();
    restore(&dir, &items[0].0).unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.join("b.txt")).unwrap(),
        "new file"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn snapshot_never_touches_head_or_the_index() {
    let dir = temp_repo("headsafe");
    let head = |d: &std::path::Path| {
        String::from_utf8(
            Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(d)
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
    };
    let before = head(&dir);
    std::fs::write(dir.join("a.txt"), "changed").unwrap();
    snapshot(&dir, "safe").unwrap();
    assert_eq!(head(&dir), before, "HEAD moved");
    let status = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(&dir)
        .output()
        .unwrap();
    let status = String::from_utf8(status.stdout).unwrap();
    assert!(status.contains(" M a.txt"), "edit still unstaged: {status}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn snapshot_outside_a_repo_reports_it() {
    let dir = std::env::temp_dir().join(format!("crew-ckpt-norepo-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let err = snapshot(&dir, "x").unwrap_err();
    assert!(err.contains("not a git repository"), "{err}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn ordinals_map_oldest_first() {
    let dir = temp_repo("ordinals");
    snapshot(&dir, "first").unwrap();
    // creatordate sorts at 1s resolution — keep the second snapshot behind it.
    std::thread::sleep(std::time::Duration::from_millis(1100));
    std::fs::write(dir.join("a.txt"), "later").unwrap();
    snapshot(&dir, "second").unwrap();
    let items = list(&dir).unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].1, "first");
    assert_eq!(items[1].1, "second");
    let _ = std::fs::remove_dir_all(&dir);
}

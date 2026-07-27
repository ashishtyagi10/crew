//! `since` against a real repository. The parsing half is unit-tested in
//! `changed`; what needs a repo is the part that cannot be faked — that the
//! tree comparison sees an untracked file, a deletion and an edit, and that a
//! task which touched nothing produces no diff at all.
use std::path::PathBuf;
use std::process::Command;

use super::since;
use crate::broker::checkpoint::worktree_tree;

/// A throwaway git repo with one committed file, isolated per test.
fn temp_repo(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "crew-changed-test-{tag}-{}-{}",
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
fn an_edit_an_addition_and_a_deletion_are_all_reported() {
    let dir = temp_repo("mixed");
    let base = worktree_tree(&dir).unwrap();

    std::fs::write(dir.join("a.txt"), "edited").unwrap();
    std::fs::write(dir.join("b.txt"), "brand new").unwrap();
    std::fs::write(dir.join("gone.txt"), "temporary").unwrap();
    let mid = worktree_tree(&dir).unwrap();
    std::fs::remove_file(dir.join("gone.txt")).unwrap();

    let mut changes = since(&dir, &base).unwrap();
    changes.sort();
    assert_eq!(
        changes,
        vec![('A', "b.txt".to_string()), ('M', "a.txt".to_string())]
    );

    // A file that appeared and vanished between two snapshots is a deletion
    // relative to the one that saw it — an agent cleaning up after itself.
    let mut back = since(&dir, &mid).unwrap();
    back.sort();
    assert_eq!(back, vec![('D', "gone.txt".to_string())]);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The trap this feature would otherwise fall into on every task: the broker
/// rewrites `.crew/session-live.md` as each reply streams, so in a repo that
/// does not gitignore it — which is every repo but crew's own — the answer to
/// "what did the task change?" would be "the transcript", forever.
#[test]
fn crews_own_session_log_is_not_reported_as_a_change() {
    let dir = temp_repo("sessionlog");
    let base = worktree_tree(&dir).unwrap();
    std::fs::create_dir_all(dir.join(".crew")).unwrap();
    std::fs::write(dir.join(".crew/session-live.md"), "## a reply").unwrap();
    assert!(
        since(&dir, &base).unwrap().is_empty(),
        "crew's own bookkeeping must not read as the user's work"
    );
    // …but a real edit alongside it still does.
    std::fs::write(dir.join("a.txt"), "edited").unwrap();
    assert_eq!(
        since(&dir, &base).unwrap(),
        vec![('M', "a.txt".to_string())]
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The common case: a question. Nothing was written, so there is nothing to
/// say — and `since` must reach that answer without running a diff at all.
#[test]
fn a_task_that_changed_nothing_reports_nothing() {
    let dir = temp_repo("clean");
    let base = worktree_tree(&dir).unwrap();
    assert!(since(&dir, &base).unwrap().is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

/// Running outside a repository is a normal way to use crew; it must be an
/// error the caller can ignore, never a panic.
#[test]
fn outside_a_repository_it_is_an_error_not_a_panic() {
    let dir = std::env::temp_dir().join(format!("crew-changed-norepo-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    assert!(since(&dir, "deadbeef").is_err());
    let _ = std::fs::remove_dir_all(&dir);
}

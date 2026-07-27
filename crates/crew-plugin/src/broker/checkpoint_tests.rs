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

/// The dedupe that makes automatic checkpoints usable: a task that changed
/// nothing must not write a restore point identical to the last one, or the
/// real ones drown in noise.
#[test]
fn an_unchanged_tree_writes_no_new_checkpoint() {
    let dir = temp_repo("auto-dedupe");
    let mut last = None;
    let first = auto_snapshot(&dir, "before: task one", &mut last).unwrap();
    assert!(first.is_some(), "the first task always snapshots");
    assert_eq!(list(&dir).unwrap().len(), 1);

    // Nothing touched the tree — no second snapshot, and `last` still holds.
    let again = auto_snapshot(&dir, "before: task two", &mut last).unwrap();
    assert_eq!(again, None, "identical tree wrote a checkpoint anyway");
    assert_eq!(list(&dir).unwrap().len(), 1);

    // A real edit is a real restore point.
    std::fs::write(dir.join("a.txt"), "two").unwrap();
    let third = auto_snapshot(&dir, "before: task three", &mut last).unwrap();
    assert!(third.is_some(), "a changed tree must snapshot");
    assert_eq!(list(&dir).unwrap().len(), 2);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Outside a repository there is nothing to pin, and that is a normal way to
/// run crew — the caller ignores this, so it must be an error and not a panic.
#[test]
fn auto_snapshot_outside_a_repo_is_an_error_not_a_panic() {
    let dir = std::env::temp_dir().join(format!("crew-ckpt-norepo-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut last = None;
    assert!(auto_snapshot(&dir, "before: x", &mut last).is_err());
    assert_eq!(last, None, "a failed snapshot must not claim a tree");
    let _ = std::fs::remove_dir_all(&dir);
}

/// An automatic feature owns its own cleanup: snapshots are now written
/// without anyone asking, so the ref space must not grow without bound.
#[test]
fn prune_keeps_only_the_newest() {
    let dir = temp_repo("auto-prune");
    for i in 0..5 {
        std::fs::write(dir.join("a.txt"), format!("{i}")).unwrap();
        snapshot(&dir, &format!("n{i}")).unwrap();
    }
    assert_eq!(list(&dir).unwrap().len(), 5);
    prune(&dir, 2);
    let left = list(&dir).unwrap();
    assert_eq!(left.len(), 2, "{left:?}");
    // Oldest-first ordering: the survivors are the LAST two written.
    assert_eq!(left[1].1, "n4", "{left:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Five checkpoints inside one second. Git commit dates cannot order these —
/// only the sequence in the ref name can, and `/restore <n>` counts on it.
#[test]
fn checkpoints_taken_in_the_same_second_keep_their_order() {
    let dir = temp_repo("auto-order");
    for i in 0..5 {
        std::fs::write(dir.join("a.txt"), format!("{i}")).unwrap();
        snapshot(&dir, &format!("n{i}")).unwrap();
    }
    let labels: Vec<String> = list(&dir).unwrap().into_iter().map(|(_, l)| l).collect();
    assert_eq!(labels, vec!["n0", "n1", "n2", "n3", "n4"], "{labels:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The sequence comes off the refs, not off memory, so a restarted broker
/// keeps counting where the last one stopped instead of colliding.
#[test]
fn the_sequence_survives_a_restart() {
    let dir = temp_repo("auto-seq");
    std::fs::write(dir.join("a.txt"), "x").unwrap();
    snapshot(&dir, "first").unwrap();
    assert_eq!(next_seq(&dir), 1);
    std::fs::write(dir.join("a.txt"), "y").unwrap();
    snapshot(&dir, "second").unwrap();
    assert_eq!(next_seq(&dir), 2, "a fresh process must not reuse a number");
    let _ = std::fs::remove_dir_all(&dir);
}

/// An undo that leaves half the change is not an undo. The agent edits a file
/// AND writes a new one; restoring put the edit back and left the new module
/// sitting there, for the user to find and delete by hand.
#[test]
fn restore_removes_what_the_task_created() {
    let dir = temp_repo("undo-created");
    snapshot(&dir, "before the task").unwrap();
    let sha = &list(&dir).unwrap()[0].0.clone();

    std::fs::write(dir.join("a.txt"), "wrecked").unwrap();
    std::fs::create_dir_all(dir.join("src/deep")).unwrap();
    std::fs::write(dir.join("src/deep/half_written.rs"), "fn oops() {").unwrap();

    let removed = restore(&dir, sha).unwrap();
    assert_eq!(removed, vec!["src/deep/half_written.rs".to_string()]);
    assert_eq!(std::fs::read_to_string(dir.join("a.txt")).unwrap(), "one");
    assert!(!dir.join("src/deep/half_written.rs").exists());
    // …and no trail of empty folders where the file used to be.
    assert!(!dir.join("src").exists(), "empty directories survived");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Files that were ALREADY there when the snapshot was taken are not part of
/// the change and must survive. This is why the removal is a set difference
/// against the snapshot and not `git clean`, which would take them.
#[test]
fn restore_keeps_files_that_predate_the_snapshot() {
    let dir = temp_repo("undo-predates");
    std::fs::write(dir.join("mine.txt"), "written before any agent ran").unwrap();
    snapshot(&dir, "before the task").unwrap();
    let sha = &list(&dir).unwrap()[0].0.clone();

    std::fs::write(dir.join("agents.txt"), "written by the task").unwrap();
    let removed = restore(&dir, sha).unwrap();

    assert_eq!(removed, vec!["agents.txt".to_string()]);
    assert!(dir.join("mine.txt").exists(), "an older file was deleted");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Build output and untracked secrets are ignored files: absent from the
/// snapshot AND from the worktree listing, so they can never be candidates.
/// Deleting somebody's `.env` because an agent ran would be unforgivable.
#[test]
fn restore_never_touches_ignored_files() {
    let dir = temp_repo("undo-ignored");
    std::fs::write(dir.join(".gitignore"), "target/\n.env\n").unwrap();
    snapshot(&dir, "before the task").unwrap();
    let sha = &list(&dir).unwrap()[0].0.clone();

    std::fs::create_dir_all(dir.join("target")).unwrap();
    std::fs::write(dir.join("target/build.o"), "artifact").unwrap();
    std::fs::write(dir.join(".env"), "SECRET=1").unwrap();

    let removed = restore(&dir, sha).unwrap();
    assert!(removed.is_empty(), "removed ignored files: {removed:?}");
    assert!(dir.join(".env").exists(), "deleted a secret");
    assert!(dir.join("target/build.o").exists(), "deleted build output");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Crew's own live transcript is written DURING the session doing the
/// restoring. Deleting it would be the tool undoing itself.
#[test]
fn restore_leaves_crews_own_transcript_alone() {
    let dir = temp_repo("undo-crewdir");
    snapshot(&dir, "before the task").unwrap();
    let sha = &list(&dir).unwrap()[0].0.clone();

    std::fs::create_dir_all(dir.join(".crew")).unwrap();
    std::fs::write(dir.join(".crew/session-live.md"), "## this session").unwrap();

    let removed = restore(&dir, sha).unwrap();
    assert!(removed.is_empty(), "{removed:?}");
    assert!(dir.join(".crew/session-live.md").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

/// A deletion nobody is told about is not an undo, it is a surprise.
#[test]
fn the_reply_names_what_it_deleted() {
    assert_eq!(removed_note(&[]), "", "a plain restore says nothing extra");
    let one = removed_note(&["src/a.rs".to_string()]);
    assert!(one.contains("removed 1 file created since"), "{one}");
    assert!(one.contains("src/a.rs"), "{one}");
    let many: Vec<String> = (0..7).map(|i| format!("f{i}.rs")).collect();
    let note = removed_note(&many);
    assert!(note.contains("removed 7 files"), "{note}");
    assert!(note.contains("f3.rs") && !note.contains("f4.rs"), "{note}");
    assert!(note.ends_with(", +3 more"), "{note}");
}

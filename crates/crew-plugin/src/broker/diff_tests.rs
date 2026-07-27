use super::*;

#[test]
fn clean_tree_reports_a_friendly_line() {
    assert_eq!(diff_report(""), "working tree clean \u{2014} no changes");
    assert_eq!(
        diff_report("   \n  \t "),
        "working tree clean \u{2014} no changes"
    );
}

#[test]
fn small_stat_passes_through_trimmed() {
    let stat = "\n a.txt | 2 +-\n 1 file changed, 1 insertion(+), 1 deletion(-)\n";
    assert_eq!(
        diff_report(stat),
        "a.txt | 2 +-\n 1 file changed, 1 insertion(+), 1 deletion(-)"
    );
}

#[test]
fn over_cap_stat_is_truncated_with_marker() {
    let long = "x".repeat(5000);
    let out = diff_report(&long);
    assert_eq!(
        out.chars().count(),
        4000 + "\n\u{2026} (diff truncated)".chars().count()
    );
    assert!(out.starts_with(&"x".repeat(4000)), "kept the head");
    assert!(
        out.ends_with("\u{2026} (diff truncated)"),
        "marked truncated: {out}"
    );
}

/// The states an agent actually leaves a repository in. `git diff --stat`
/// compares the working tree to the INDEX, so both of these reported "working
/// tree clean — no changes" while the agent had just written a file.
#[test]
fn a_new_file_and_a_staged_edit_both_show() {
    let dir = temp_repo("states");
    std::fs::write(dir.join("new.rs"), "brand new").unwrap();
    std::fs::write(dir.join("a.txt"), "edited").unwrap();
    run(&dir, &["add", "a.txt"]); // staged, so `git diff` excluded it

    let stat = worktree_stat(&dir).unwrap();
    assert!(stat.contains("new.rs"), "untracked file missing: {stat}");
    assert!(stat.contains("a.txt"), "staged edit missing: {stat}");
    assert!(
        !diff_report(&stat).starts_with("working tree clean"),
        "reported clean with two changed files: {stat}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The end-of-task note filters crew's own transcript out of "what changed";
/// `/diff` must agree, or the note names a command that contradicts it.
#[test]
fn crews_own_files_are_excluded_here_too() {
    let dir = temp_repo("crewdir");
    std::fs::create_dir_all(dir.join(".crew")).unwrap();
    std::fs::write(dir.join(".crew/session-live.md"), "## a reply").unwrap();
    assert_eq!(
        diff_report(&worktree_stat(&dir).unwrap()),
        "working tree clean \u{2014} no changes"
    );
    // …by being EXCLUDED, not by nothing being visible. Without this the test
    // also passes against the old implementation, which saw no untracked file
    // of any kind — a pass for the wrong reason is not cover.
    std::fs::write(dir.join("real.rs"), "the user's work").unwrap();
    let stat = worktree_stat(&dir).unwrap();
    assert!(stat.contains("real.rs"), "{stat}");
    assert!(!stat.contains(".crew"), "{stat}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// An untouched checkout still says so — the point is to stop reporting clean
/// when it is not, not to start reporting changes when there are none.
#[test]
fn an_untouched_tree_is_still_clean() {
    let dir = temp_repo("clean");
    assert_eq!(
        diff_report(&worktree_stat(&dir).unwrap()),
        "working tree clean \u{2014} no changes"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// A repository with no commits has no HEAD to compare against. `git rev-parse
/// HEAD` fails there, and the first files someone writes must show as
/// additions rather than as "diff failed".
#[test]
fn a_repository_with_no_commits_still_diffs() {
    let dir = std::env::temp_dir().join(format!(
        "crew-diff-unborn-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0),
    ));
    std::fs::create_dir_all(&dir).unwrap();
    run(&dir, &["init", "-q"]);
    std::fs::write(dir.join("first.rs"), "hello").unwrap();
    let stat = worktree_stat(&dir).unwrap();
    assert!(stat.contains("first.rs"), "unborn branch: {stat}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn outside_a_repository_it_is_an_error_not_a_panic() {
    let dir = std::env::temp_dir().join(format!("crew-diff-norepo-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    assert!(worktree_stat(&dir).is_err());
    let _ = std::fs::remove_dir_all(&dir);
}

fn run(dir: &std::path::Path, args: &[&str]) {
    assert!(std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .unwrap()
        .success());
}

/// A throwaway git repo with one committed file, isolated per test.
fn temp_repo(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "crew-diff-test-{tag}-{}-{}",
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
        run(&dir, args);
    }
    std::fs::write(dir.join("a.txt"), "one").unwrap();
    run(&dir, &["add", "-A"]);
    run(&dir, &["commit", "-q", "-m", "init"]);
    dir
}

use super::*;
use std::path::{Path, PathBuf};

/// A throwaway git repo with identity configured (commits work in CI).
fn repo(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("crew-gitmsg-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    for args in [
        vec!["init", "-q"],
        vec!["config", "user.email", "t@t"],
        vec!["config", "user.name", "t"],
    ] {
        assert!(git(&d, &args).is_ok(), "git {args:?}");
    }
    d
}

fn seed_commit(d: &Path) {
    std::fs::write(d.join("a.txt"), "one\n").unwrap();
    git(d, &["add", "."]).unwrap();
    git(d, &["commit", "-qm", "seed"]).unwrap();
}

#[test]
fn pick_diff_prefers_staged_and_reports_clean() {
    let d = repo("pick");
    seed_commit(&d);
    assert_eq!(pick_diff(&d).unwrap(), None, "clean tree");
    // unstaged edit
    std::fs::write(d.join("a.txt"), "two\n").unwrap();
    let (diff, staged) = pick_diff(&d).unwrap().unwrap();
    assert!(!staged && diff.contains("two"));
    // staging it flips the source (and staged wins over a further edit)
    git(&d, &["add", "."]).unwrap();
    std::fs::write(d.join("a.txt"), "three\n").unwrap();
    let (diff, staged) = pick_diff(&d).unwrap().unwrap();
    assert!(staged, "staged diff wins");
    assert!(diff.contains("two") && !diff.contains("three"));
}

#[test]
fn pick_diff_errs_outside_a_repo() {
    let d = std::env::temp_dir().join(format!("crew-gitmsg-norepo-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    assert!(pick_diff(&d).is_err());
}

// The message's author is elected by the MODEL (`elect::elect`), with a
// deterministic roster-first fallback — see `elect_tests`; the keyword
// election it replaced is gone.

#[test]
fn commit_prompt_carries_the_diff_and_the_rules() {
    let p = commit_prompt("+ fn new_thing() {}");
    assert!(p.contains("+ fn new_thing() {}"));
    let lower = p.to_lowercase();
    assert!(lower.contains("conventional"), "names the style: {p}");
    assert!(lower.contains("message"), "asks for the message only");
}

#[test]
fn clean_message_strips_fences_and_labels_but_keeps_the_body() {
    assert_eq!(
        clean_message("```\nfeat: add x\n\nlonger body\n```"),
        "feat: add x\n\nlonger body"
    );
    assert_eq!(
        clean_message("Commit message:\nfix(app): correct y"),
        "fix(app): correct y"
    );
    assert_eq!(clean_message("  feat: plain  "), "feat: plain");
    assert_eq!(clean_message(""), "");
}

#[test]
fn do_commit_creates_the_commit_for_both_modes() {
    // staged mode: -m commits what's in the index
    let d = repo("apply-staged");
    seed_commit(&d);
    std::fs::write(d.join("a.txt"), "two\n").unwrap();
    git(&d, &["add", "."]).unwrap();
    do_commit(&d, "feat: staged change", true).unwrap();
    let subj = git(&d, &["log", "-1", "--format=%s"]).unwrap();
    assert_eq!(subj.trim(), "feat: staged change");
    // unstaged mode: -am picks up tracked edits
    let d = repo("apply-unstaged");
    seed_commit(&d);
    std::fs::write(d.join("a.txt"), "three\n").unwrap();
    do_commit(&d, "fix: unstaged change", false).unwrap();
    let subj = git(&d, &["log", "-1", "--format=%s"]).unwrap();
    assert_eq!(subj.trim(), "fix: unstaged change");
}

/// The file an agent just wrote. `git diff` cannot see a path git has never
/// been told about, so `/commit` drafted a message that never mentioned the
/// new module and `/review` reviewed a change with its main file missing.
#[test]
fn a_new_file_is_part_of_the_change() {
    let d = repo("untracked-diff");
    seed_commit(&d);
    std::fs::write(d.join("new.rs"), "fn added() {}\n").unwrap();
    let (diff, staged) = pick_diff(&d).unwrap().unwrap();
    assert!(!staged, "nothing was staged");
    assert!(
        diff.contains("new.rs"),
        "new file missing from diff: {diff}"
    );
    assert!(diff.contains("fn added"), "its contents too: {diff}");
}

/// …and `apply` commits it. `git commit -am` stages tracked modifications and
/// nothing else, so the new file stayed behind while the pane reported a
/// successful commit — the message described work that was not in it.
#[test]
fn apply_commits_the_new_file_it_described() {
    let d = repo("untracked-apply");
    seed_commit(&d);
    std::fs::write(d.join("new.rs"), "fn added() {}\n").unwrap();
    std::fs::write(d.join("a.txt"), "edited\n").unwrap();
    do_commit(&d, "feat: add the module", false).unwrap();
    let files = git(&d, &["show", "--name-only", "--format=", "HEAD"]).unwrap();
    assert!(files.contains("new.rs"), "new file not committed: {files}");
    assert!(files.contains("a.txt"), "edit not committed: {files}");
    assert!(
        git(&d, &["status", "--porcelain"])
            .unwrap()
            .trim()
            .is_empty(),
        "something was left behind: {:?}",
        git(&d, &["status", "--porcelain"])
    );
}

/// Staging is a statement about which change you mean. A partial stage must
/// still commit exactly that, or `/commit` would quietly widen the commit
/// someone had deliberately narrowed.
#[test]
fn a_partial_stage_is_still_respected() {
    let d = repo("partial");
    seed_commit(&d);
    std::fs::write(d.join("a.txt"), "meant\n").unwrap();
    git(&d, &["add", "a.txt"]).unwrap();
    std::fs::write(d.join("other.rs"), "not meant yet\n").unwrap();

    let (diff, staged) = pick_diff(&d).unwrap().unwrap();
    assert!(staged, "a staged change wins");
    assert!(!diff.contains("other.rs"), "described too much: {diff}");
    do_commit(&d, "feat: only what was staged", true).unwrap();
    let files = git(&d, &["show", "--name-only", "--format=", "HEAD"]).unwrap();
    assert!(!files.contains("other.rs"), "committed too much: {files}");
}

/// Crew's own transcript is not the user's work — excluded from the message
/// AND from what `apply` stages, or the commit would contain a file the
/// message never mentioned.
#[test]
fn crews_own_files_are_neither_described_nor_committed() {
    let d = repo("crewdir");
    seed_commit(&d);
    std::fs::create_dir_all(d.join(".crew")).unwrap();
    std::fs::write(d.join(".crew/session-live.md"), "## a reply\n").unwrap();
    assert_eq!(
        pick_diff(&d).unwrap(),
        None,
        "only crew's own files changed"
    );

    std::fs::write(d.join("real.rs"), "the user's work\n").unwrap();
    let (diff, _) = pick_diff(&d).unwrap().unwrap();
    assert!(
        diff.contains("real.rs") && !diff.contains(".crew"),
        "{diff}"
    );
    do_commit(&d, "feat: real work", false).unwrap();
    let files = git(&d, &["show", "--name-only", "--format=", "HEAD"]).unwrap();
    assert!(
        files.contains("real.rs") && !files.contains(".crew"),
        "{files}"
    );
}

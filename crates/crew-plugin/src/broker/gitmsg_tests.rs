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

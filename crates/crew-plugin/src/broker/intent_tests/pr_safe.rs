//! "open a PR" pushes to a real remote, so what may open it is narrow: only
//! "apply", only for the commit the draft described, and a failed push leaves
//! the user where they were (fixtures in `pr`).
use super::capability::git;
use super::pr::{ask, commit_b, fake_gh, pending, repo, REPLY};
use super::*;

#[test]
fn only_apply_opens_it_and_a_yes_does_not() {
    let _g = testenv::mock_with_specialists(REPLY, testenv::TRIO);
    let (dir, _o) = repo("yes");
    git(&dir, &["checkout", "-qb", "feat/b"]);
    commit_b(&dir);
    let gh = fake_gh("yes", 0, 0);
    let mut s = Session::new();
    ask(&mut s, "open a PR");
    for loose in ["yes", "do it", "go ahead", "ship it"] {
        ask(&mut s, loose);
        assert!(!gh.argv().contains("pr create"), "{loose} opened it");
    }
    crate::broker::gitpr::drop_pr(&s);
    assert!(!pending(&s), "/clear and rewind call this");
}

#[test]
fn a_commit_after_the_draft_is_not_pushed_under_it() {
    let _g = testenv::mock_with_specialists(REPLY, testenv::TRIO);
    let (dir, origin) = repo("moved");
    git(&dir, &["checkout", "-qb", "feat/b"]);
    commit_b(&dir);
    let gh = fake_gh("moved", 0, 0);
    let mut s = Session::new();
    ask(&mut s, "open a PR");
    std::fs::write(dir.join("c.txt"), "c\n").unwrap();
    git(&dir, &["add", "c.txt"]);
    git(&dir, &["commit", "-qm", "feat: add c"]);
    let evs = ask(&mut s, "apply");
    assert!(any_text(&evs, "new commits since the draft"), "{evs:?}");
    assert!(!gh.argv().contains("pr create"));
    let heads = git(&origin, &["for-each-ref", "--format=%(refname:short)"]);
    assert!(!heads.contains("feat/b"), "nothing pushed: {heads}");
}

#[test]
fn a_failed_push_puts_the_user_back_on_the_base() {
    let _g = testenv::mock_with_specialists(REPLY, testenv::TRIO);
    let (dir, origin) = repo("nopush");
    commit_b(&dir);
    let gh = fake_gh("nopush", 0, 0);
    let mut s = Session::new();
    ask(&mut s, "open a PR");
    assert!(pending(&s));
    std::fs::remove_dir_all(&origin).unwrap();
    let evs = ask(&mut s, "apply");
    assert!(any_text(&evs, "push failed"), "{evs:?}");
    assert_eq!(git(&dir, &["rev-parse", "--abbrev-ref", "HEAD"]), "main");
    let branches = git(&dir, &["branch", "--format=%(refname:short)"]);
    assert_eq!(branches.trim(), "main", "the made branch is gone");
    assert!(!gh.argv().contains("pr create"));
}

#[test]
fn main_and_master_without_origin_head_is_refused() {
    let _g = testenv::mock_with_specialists(REPLY, testenv::TRIO);
    let (dir, _o) = repo("both-bases");
    git(&dir, &["branch", "master"]);
    git(&dir, &["checkout", "-qb", "feat/b"]);
    commit_b(&dir);
    let _gh = fake_gh("both-bases", 0, 0);
    let mut s = Session::new();
    let evs = ask(&mut s, "open a PR");
    assert!(any_text(&evs, "can't tell the default branch"), "{evs:?}");
    assert!(!pending(&s));
}

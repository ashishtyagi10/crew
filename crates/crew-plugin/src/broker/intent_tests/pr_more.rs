//! "open a PR", the refusals and the interplay with a pending commit and the
//! review (fixtures in `pr`).
use super::capability::{git, route_on};
use super::pr::{ask, commit_b, fake_gh, pending, repo, REPLY};
use super::*;
use crate::broker::gitmsg::PendingCommit;

#[test]
fn unmet_preconditions_are_one_line_and_no_draft() {
    let _g = testenv::mock_with_specialists(REPLY, testenv::TRIO);
    let (dir, _o) = repo("pre");
    let _gh = fake_gh("pre", 0, 0);
    let mut s = Session::new();
    // nothing ahead
    let evs = ask(&mut s, "open a PR");
    assert!(any_text(&evs, "nothing ahead of main"), "{evs:?}");
    // dirty tree
    commit_b(&dir);
    std::fs::write(dir.join("a.txt"), "dirty\n").unwrap();
    let evs = ask(&mut s, "open a PR");
    assert!(any_text(&evs, "commit first"), "{evs:?}");
    assert!(any_text(&evs, "commit this"), "{evs:?}");
    assert!(!pending(&s));
    // gh missing, then not signed in
    git(&dir, &["checkout", "-q", "a.txt"]);
    std::env::set_var("CREW_GH", "/nonexistent/gh");
    let evs = ask(&mut s, "open a PR");
    assert!(any_text(&evs, "not installed"), "{evs:?}");
    let _gh = fake_gh("pre2", 1, 0);
    let evs = ask(&mut s, "open a PR");
    assert!(any_text(&evs, "not signed in"), "{evs:?}");
    // no origin
    git(&dir, &["remote", "remove", "origin"]);
    let _gh = fake_gh("pre3", 0, 0);
    let evs = ask(&mut s, "open a PR");
    assert!(any_text(&evs, "no `origin` remote"), "{evs:?}");
    assert!(!pending(&s));
}

#[test]
fn a_failing_gh_reports_its_stderr_and_still_clears_the_proposal() {
    let _g = testenv::mock_with_specialists(REPLY, testenv::TRIO);
    let (dir, _o) = repo("fail");
    git(&dir, &["checkout", "-qb", "feat/b"]);
    commit_b(&dir);
    let _gh = fake_gh("fail", 0, 1);
    let mut s = Session::new();
    ask(&mut s, "open a PR");
    let evs = ask(&mut s, "apply");
    assert!(any_text(&evs, "gh pr create failed: boom"), "{evs:?}");
    assert!(!pending(&s));
}

#[test]
fn drafting_one_clears_the_other_and_apply_still_commits() {
    let _g = testenv::mock_with_specialists(REPLY, testenv::TRIO);
    let (dir, _o) = repo("both");
    git(&dir, &["checkout", "-qb", "feat/b"]);
    commit_b(&dir);
    let gh = fake_gh("both", 0, 0);
    let mut s = Session::new();
    *s.commit.lock().unwrap() = Some(PendingCommit {
        message: "old".into(),
        staged: false,
    });
    ask(&mut s, "open a PR");
    assert!(pending(&s) && s.commit.lock().unwrap().is_none());
    // a commit draft drops the PR…
    std::fs::write(dir.join("a.txt"), "two\n").unwrap();
    route_on(&mut s, "commit this", &|_: &str| Ok("SHAPE: commit".into()));
    assert!(!pending(&s) && s.commit.lock().unwrap().is_some());
    // …and "apply" then commits, not opens a PR.
    let before = git(&dir, &["rev-list", "--count", "HEAD"]);
    ask(&mut s, "apply");
    let after = git(&dir, &["rev-list", "--count", "HEAD"]);
    assert_eq!(
        after.parse::<u32>().unwrap(),
        before.parse::<u32>().unwrap() + 1
    );
    assert!(!gh.argv().contains("pr create"));
}

#[test]
fn review_my_pr_stays_a_review() {
    let _g = testenv::mock_with_specialists("no findings", testenv::TRIO);
    let (dir, _o) = repo("rev");
    git(&dir, &["checkout", "-qb", "feat/b"]);
    commit_b(&dir);
    let gh = fake_gh("rev", 0, 0);
    let mut s = Session::new();
    let evs = route_on(
        &mut s,
        "review my PR",
        &|_: &str| Ok("SHAPE: review".into()),
    );
    assert!(!any_text(&evs, "drafting a pull request"), "{evs:?}");
    assert!(any_text(&evs, "nothing to review"), "{evs:?}");
    assert!(!pending(&s) && gh.argv().is_empty());
}

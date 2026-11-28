//! "open a PR": drafted from the branch's commits, opened only by the exact
//! "apply". A bare repo is the `origin`, so pushes work offline; a fake `gh`
//! script (`CREW_GH`) records its argv and stands in for GitHub.
use super::capability::{git, route_on};
use super::*;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// Sets `CREW_GH` for the test and removes it after (the env guard's lock is
/// held for the whole test, so nothing else sees it).
pub(super) struct Gh {
    pub(super) log: PathBuf,
    pub(super) dir: PathBuf,
}

impl Drop for Gh {
    fn drop(&mut self) {
        std::env::remove_var("CREW_GH");
    }
}

fn tmp(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("crew-pr-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// A fake gh: `auth_rc` is what `gh auth status` exits with, `create_rc` what
/// `gh pr create` exits with (stderr "boom" when non-zero).
pub(super) fn fake_gh(tag: &str, auth_rc: i32, create_rc: i32) -> Gh {
    let dir = tmp(&format!("gh-{tag}"));
    let log = dir.join("argv.log");
    let script = dir.join("gh");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\necho \"$@\" >> '{log}'\n\
             if [ \"$1\" = auth ]; then exit {auth_rc}; fi\n\
             prev=\n\
             for a in \"$@\"; do if [ \"$prev\" = --body-file ]; then cp \"$a\" '{body}'; fi; prev=$a; done\n\
             if [ {create_rc} -ne 0 ]; then echo boom >&2; exit {create_rc}; fi\n\
             echo https://github.com/o/r/pull/7\n",
            log = log.display(),
            body = dir.join("body").display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::env::set_var("CREW_GH", &script);
    Gh { log, dir }
}

impl Gh {
    pub(super) fn argv(&self) -> String {
        std::fs::read_to_string(&self.log).unwrap_or_default()
    }
}

/// The project dir as a repo on `main` with a bare origin that has `main`.
pub(super) fn repo(tag: &str) -> (PathBuf, PathBuf) {
    let dir = PathBuf::from(std::env::var("CREW_PROJECT_DIR").unwrap());
    let origin = tmp(&format!("origin-{tag}"));
    git(&origin, &["init", "-q", "--bare"]);
    for args in [
        vec!["init", "-q", "-b", "main"],
        vec!["config", "user.email", "t@t"],
        vec!["config", "user.name", "t"],
    ] {
        git(&dir, &args);
    }
    std::fs::write(dir.join("a.txt"), "one\n").unwrap();
    git(&dir, &["add", "a.txt"]);
    git(&dir, &["commit", "-qm", "seed"]);
    git(&dir, &["remote", "add", "origin", origin.to_str().unwrap()]);
    git(&dir, &["push", "-q", "-u", "origin", "main"]);
    (dir, origin)
}

pub(super) fn commit_b(dir: &Path) {
    std::fs::write(dir.join("b.txt"), "b\n").unwrap();
    git(dir, &["add", "b.txt"]);
    git(dir, &["commit", "-qm", "feat: add b"]);
}

pub(super) fn ask(session: &mut Session, task: &str) -> Vec<PluginEvent> {
    // A saboteur router: the gates must win before it is asked.
    route_on(session, task, &|_: &str| Ok("SHAPE: swarm".to_string()))
}

pub(super) const REPLY: &str = "feat(app): add b\n\nAdds b.txt.";

pub(super) fn pending(s: &Session) -> bool {
    crate::broker::gitpr::pending(s)
}

fn heads(origin: &Path) -> String {
    git(
        origin,
        &["for-each-ref", "--format=%(refname:short)", "refs/heads"],
    )
}

#[test]
fn a_feature_branch_drafts_then_apply_pushes_and_opens() {
    let _g = testenv::mock_with_specialists(REPLY, testenv::TRIO);
    let (dir, origin) = repo("feat");
    git(&dir, &["checkout", "-qb", "feat/b"]);
    commit_b(&dir);
    let gh = fake_gh("feat", 0, 0);
    let mut s = Session::new();
    let evs = ask(&mut s, "open a PR");
    assert!(any_text(&evs, "feat/b → main"), "{evs:?}");
    assert!(any_text(&evs, "git push -u origin feat/b"), "{evs:?}");
    assert!(
        any_text(&evs, "gh pr create --base main --head feat/b"),
        "{evs:?}"
    );
    assert!(any_text(&evs, "apply"), "{evs:?}");
    assert!(pending(&s));
    // A draft pushes nothing and opens nothing.
    assert_eq!(heads(&origin), "main");
    assert!(!gh.argv().contains("pr create"));
    let evs = ask(&mut s, "apply");
    assert!(any_text(&evs, "https://github.com/o/r/pull/7"), "{evs:?}");
    assert!(!pending(&s), "apply consumed the proposal");
    assert!(heads(&origin).contains("feat/b"));
    let argv = gh.argv();
    assert!(
        argv.contains("pr create --base main --head feat/b --title feat(app): add b --body-file"),
        "{argv}"
    );
    assert_eq!(
        std::fs::read_to_string(gh.dir.join("body")).unwrap(),
        "Adds b.txt."
    );
}

#[test]
fn on_the_base_a_branch_is_proposed_and_the_base_is_never_pushed() {
    let _g = testenv::mock_with_specialists(REPLY, testenv::TRIO);
    let (dir, origin) = repo("base");
    let origin_main = git(&origin, &["rev-parse", "main"]);
    commit_b(&dir);
    let gh = fake_gh("base", 0, 0);
    let mut s = Session::new();
    let evs = ask(&mut s, "make a pull request");
    assert!(any_text(&evs, "feat/add-b → main"), "{evs:?}");
    assert!(any_text(&evs, "git checkout -b feat/add-b"), "{evs:?}");
    assert!(any_text(&evs, "main is not pushed"), "{evs:?}");
    let evs = ask(&mut s, "apply");
    assert!(any_text(&evs, "pull/7"), "{evs:?}");
    assert_eq!(heads(&origin), "feat/add-b\nmain");
    assert_eq!(
        git(&origin, &["rev-parse", "main"]),
        origin_main,
        "base untouched"
    );
    assert_eq!(
        git(&dir, &["rev-parse", "--abbrev-ref", "HEAD"]),
        "feat/add-b"
    );
    let argv = gh.argv();
    assert!(!argv.contains("origin main"), "{argv}");
    assert!(argv.contains("--head feat/add-b"), "{argv}");
}

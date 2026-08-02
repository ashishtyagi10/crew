//! Plain-language parity for the retired `/commit`, `/review`, `/standup`
//! and `/resume` — a natural phrasing reaches each capability through the
//! router — and the commit HUMAN GATE: classification alone can draft a
//! commit message but can never create a commit; only the user's own
//! conversational confirm ("apply") does, and that confirm is matched
//! deterministically, never by the model.
use super::*;
use std::path::{Path, PathBuf};

/// [`route_stubbed`], but on a caller-owned session — the commit gate spans
/// two sends (draft, then confirm), which must share the pending proposal.
fn route_on(session: &mut Session, task: &str, call: Classifier) -> Vec<PluginEvent> {
    let mut evs = Vec::new();
    route_with(
        task,
        Some(call),
        session,
        &crate::broker::tick::noop_tick_emit(),
        &mut |ev| {
            evs.push(ev);
            Ok(())
        },
    )
    .unwrap();
    evs
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// Turn the guard's fresh project dir into a git repo with one seed commit
/// and one uncommitted edit — the diff a commit message would describe.
fn seed_repo() -> PathBuf {
    let dir = PathBuf::from(std::env::var("CREW_PROJECT_DIR").unwrap());
    for args in [
        vec!["init", "-q"],
        vec!["config", "user.email", "t@t"],
        vec!["config", "user.name", "t"],
    ] {
        git(&dir, &args);
    }
    std::fs::write(dir.join("a.txt"), "one\n").unwrap();
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-qm", "seed"]);
    std::fs::write(dir.join("a.txt"), "two\n").unwrap();
    dir
}

fn commits(dir: &Path) -> u32 {
    git(dir, &["rev-list", "--count", "HEAD"]).parse().unwrap()
}

#[test]
fn a_commit_phrasing_drafts_and_gates_but_never_commits() {
    let _g = testenv::mock_with_specialists("feat: change a.txt", testenv::TRIO);
    let dir = seed_repo();
    let mut session = Session::new();
    let call = |_: &str| Ok("SHAPE: commit".to_string());
    let evs = route_on(&mut session, "commit this", &call);
    // The capability was reached: a draft was announced and proposed…
    assert!(any_text(&evs, "drafting a commit message"), "{evs:?}");
    assert!(any_text(&evs, "feat: change a.txt"), "{evs:?}");
    // …the gate prompt teaches the conversational confirm…
    assert!(any_text(&evs, "apply"), "{evs:?}");
    // …and NOTHING was committed: same commit count, tree still dirty.
    assert_eq!(commits(&dir), 1, "classification alone created a commit");
    assert!(!git(&dir, &["status", "--porcelain"]).is_empty());
    let pending = session.commit.lock().unwrap();
    assert!(pending.is_some(), "the proposal is held for the confirm");
}

#[test]
fn the_conversational_apply_is_deterministic_not_classified() {
    let _g = testenv::mock_with_specialists("feat: change a.txt", testenv::TRIO);
    let dir = seed_repo();
    let mut session = Session::new();
    let draft = |_: &str| Ok("SHAPE: commit".to_string());
    route_on(&mut session, "commit this", &draft);
    assert_eq!(commits(&dir), 1);
    // The confirm send: the classifier is a saboteur that would misroute it —
    // the gate must win BEFORE any model call, so the commit happens anyway.
    let saboteur = |_: &str| Ok("SHAPE: swarm".to_string());
    let evs = route_on(&mut session, "apply", &saboteur);
    assert_eq!(commits(&dir), 2, "the user's confirm creates the commit");
    assert!(any_text(&evs, "feat: change a.txt"), "{evs:?}");
    let pending = session.commit.lock().unwrap();
    assert!(pending.is_none(), "the proposal was consumed");
}

#[test]
fn apply_with_nothing_pending_is_a_plain_message_not_a_commit() {
    let _g = testenv::mock_with_specialists("ok\n@done", testenv::TRIO);
    let dir = seed_repo();
    let mut session = Session::new();
    // No pending proposal: "apply" must go through classification like any
    // other message (here: a reply), and must not touch git.
    let call = |_: &str| Ok("SHAPE: reply".to_string());
    let evs = route_on(&mut session, "apply", &call);
    assert!(any_text(&evs, "starting with planner"), "{evs:?}");
    assert_eq!(commits(&dir), 1, "no proposal, so nothing to apply");
}

#[test]
fn a_review_phrasing_reaches_the_review_capability() {
    let _g = testenv::mock_with_specialists("no findings", testenv::TRIO);
    seed_repo();
    let mut session = Session::new();
    let call = |_: &str| Ok("SHAPE: review".to_string());
    let evs = route_on(&mut session, "look over my changes", &call);
    assert!(any_text(&evs, "reviewing the"), "{evs:?}");
    assert!(any_text(&evs, "no findings"), "{evs:?}");
}

#[test]
fn a_standup_phrasing_reaches_the_standup_capability() {
    let _g = testenv::mock_with_specialists("Done: shipped the seed", testenv::TRIO);
    seed_repo();
    let mut session = Session::new();
    let call = |_: &str| Ok("SHAPE: standup".to_string());
    let evs = route_on(&mut session, "what did I ship this week?", &call);
    assert!(any_text(&evs, "drafting a standup"), "{evs:?}");
    assert!(any_text(&evs, "Done: shipped the seed"), "{evs:?}");
}

#[test]
fn a_resume_phrasing_restores_the_previous_session() {
    let _g = testenv::mock_with_specialists("ok\n@done", testenv::TRIO);
    let dir = PathBuf::from(std::env::var("CREW_PROJECT_DIR").unwrap());
    std::fs::write(
        dir.join(".crew").join("last-session.md"),
        "user: fix the parser\ncoder: done, parser fixed\n",
    )
    .unwrap();
    let mut session = Session::new();
    let call = |_: &str| Ok("SHAPE: resume".to_string());
    let evs = route_on(&mut session, "pick up where we left off", &call);
    assert!(any_text(&evs, "restored"), "{evs:?}");
    assert!(
        session
            .resume
            .lock()
            .unwrap()
            .as_deref()
            .is_some_and(|p| p.contains("parser fixed")),
        "the previous session is loaded for the next task"
    );
}

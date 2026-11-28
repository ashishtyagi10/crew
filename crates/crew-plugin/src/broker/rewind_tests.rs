use super::*;

use std::path::PathBuf;
use std::process::Command;

pub(super) fn repo(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "crew-rewind-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
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
    std::fs::write(dir.join("a.txt"), "one\n").unwrap();
    for args in [&["add", "-A"][..], &["commit", "-qm", "init"]] {
        assert!(Command::new("git")
            .args(args)
            .current_dir(&dir)
            .status()
            .unwrap()
            .success());
    }
    dir
}

pub(super) fn tree(dir: &Path) -> String {
    super::super::checkpoint::worktree_tree(dir).unwrap()
}

/// A session whose task `id` began now on the tree `dir` holds.
pub(super) fn began(session: &Session, dir: &Path, id: u64, at_ms: u64) {
    let mut log = lock(&session.rewind);
    log.begin(id, at_ms);
    log.began_on(id, &tree(dir));
}

/// Ask, and — when `go` — confirm with the stamp that ask answered with, as
/// the pane does.
fn ask(session: &Session, dir: &Path, running: usize, id: u64, go: bool) -> PluginEvent {
    let asked = ask_seen(session, dir, running, (id, false), "");
    let seen = match &asked {
        PluginEvent::Rewind { stamp, .. } => stamp.clone(),
        _ => String::new(),
    };
    if go {
        ask_seen(session, dir, running, (id, true), &seen)
    } else {
        asked
    }
}

/// One Rewind command with an explicit `seen`.
pub(super) fn ask_seen(
    session: &Session,
    dir: &Path,
    running: usize,
    (id, go): (u64, bool),
    seen: &str,
) -> PluginEvent {
    let mut got = Vec::new();
    handle(session, dir, running, (id, go, seen), &mut |ev| {
        got.push(ev);
        Ok(())
    })
    .unwrap();
    got.pop().expect("one answer")
}

pub(super) fn answer(ev: PluginEvent) -> (u32, bool, bool, String) {
    match ev {
        PluginEvent::Rewind {
            files,
            outside,
            applied,
            error,
            ..
        } => (files, outside, applied, error),
        other => panic!("not a rewind answer: {other:?}"),
    }
}

#[test]
fn asking_counts_the_files_that_would_go_back_and_touches_nothing() {
    let dir = repo("count");
    let s = Session::default();
    began(&s, &dir, 1, 10);
    std::fs::write(dir.join("a.txt"), "two\n").unwrap();
    std::fs::write(dir.join("b.txt"), "new\n").unwrap();
    lock(&s.rewind).ended_on(1, tree(&dir));
    let (files, outside, applied, error) = answer(ask(&s, &dir, 0, 1, false));
    assert_eq!(
        (files, outside, applied, error.as_str()),
        (2, false, false, "")
    );
    assert_eq!(std::fs::read_to_string(dir.join("a.txt")).unwrap(), "two\n");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn confirming_puts_the_tree_back_and_pins_the_one_it_replaced() {
    let dir = repo("apply");
    let s = Session::default();
    began(&s, &dir, 1, 10);
    std::fs::write(dir.join("a.txt"), "two\n").unwrap();
    std::fs::write(dir.join("b.txt"), "new\n").unwrap();
    let (files, _, applied, error) = answer(ask(&s, &dir, 0, 1, true));
    assert!(applied && files == 2, "{error}");
    assert_eq!(std::fs::read_to_string(dir.join("a.txt")).unwrap(), "one\n");
    assert!(!dir.join("b.txt").exists(), "a file the task made stayed");
    let items = super::super::undo::numbered(&dir);
    assert_eq!(items[0].1, "before rewind to #1", "`undo #1` is the pin");
    // "undo that" would therefore undo the rewind: the pinned tree has the edit.
    super::super::checkpoint::restore(&dir, &items[0].0).unwrap();
    assert_eq!(std::fs::read_to_string(dir.join("a.txt")).unwrap(), "two\n");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_thread_keeps_only_what_was_answered_before_the_task() {
    let dir = repo("thread");
    let s = Session::default();
    super::super::thread::record(&s.thread, "first", Some("one".into()));
    std::thread::sleep(std::time::Duration::from_millis(5));
    let at = super::super::thread::now_ms();
    std::thread::sleep(std::time::Duration::from_millis(5));
    super::super::thread::record(&s.thread, "second", Some("two".into()));
    began(&s, &dir, 2, at);
    let (files, _, applied, _) = answer(ask(&s, &dir, 0, 2, true));
    assert!(applied);
    assert_eq!(
        files, 0,
        "nothing changed: the conversation alone goes back"
    );
    let t = super::super::thread::lock(&s.thread);
    let asks: Vec<_> = t.turns().map(|t| t.asked.as_str()).collect();
    assert_eq!(asks, ["first"]);
    // And no checkpoint was written for a rewind that moved no file.
    assert!(super::super::checkpoint::list(&dir).unwrap().is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_running_task_refuses_and_nothing_moves() {
    let dir = repo("running");
    let s = Session::default();
    began(&s, &dir, 1, 10);
    std::fs::write(dir.join("a.txt"), "two\n").unwrap();
    let (_, _, applied, error) = answer(ask(&s, &dir, 1, 1, true));
    assert!(!applied);
    assert_eq!(error, "stop it first (/stop)");
    assert_eq!(std::fs::read_to_string(dir.join("a.txt")).unwrap(), "two\n");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_task_the_broker_does_not_know_is_an_error_not_a_zero() {
    let dir = repo("unknown");
    let s = Session::default();
    let (_, _, applied, error) = answer(ask(&s, &dir, 0, 9, false));
    assert!(!applied && error.contains("#9"), "{error}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn edits_made_outside_crew_are_named_and_crews_own_are_not() {
    let dir = repo("outside");
    let s = Session::default();
    began(&s, &dir, 1, 10);
    std::fs::write(dir.join("a.txt"), "task edit\n").unwrap();
    lock(&s.rewind).ended_on(1, tree(&dir));
    let (_, outside, _, _) = answer(ask(&s, &dir, 0, 1, false));
    assert!(!outside, "the task's own edit is not outside");
    std::fs::write(dir.join("a.txt"), "my edit\n").unwrap();
    let (files, outside, _, _) = answer(ask(&s, &dir, 0, 1, false));
    assert!(outside && files == 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_edit_between_two_tasks_counts_as_outside_too() {
    let dir = repo("gap");
    let s = Session::default();
    began(&s, &dir, 1, 10);
    lock(&s.rewind).ended_on(1, tree(&dir));
    std::fs::write(dir.join("a.txt"), "typed between\n").unwrap();
    began(&s, &dir, 2, 20);
    lock(&s.rewind).ended_on(2, tree(&dir)); // so only the gap can say "outside"
    let (_, outside, _, _) = answer(ask(&s, &dir, 0, 1, false));
    assert!(outside);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_rewound_task_and_those_after_it_are_forgotten() {
    let dir = repo("forget");
    let s = Session::default();
    for id in 1..=3 {
        began(&s, &dir, id, id * 10);
    }
    answer(ask(&s, &dir, 0, 2, true));
    let log = lock(&s.rewind);
    assert!(log.get(1).is_some() && log.get(2).is_none() && log.get(3).is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

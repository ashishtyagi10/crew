use super::*;

use std::path::{Path, PathBuf};
use std::process::Command;

/// A throwaway repo with one committed file and a checkpoint taken over it.
fn repo(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "crew-undo-{tag}-{}-{}",
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
    std::fs::write(dir.join("a.txt"), "before\n").unwrap();
    for args in [&["add", "-A"][..], &["commit", "-qm", "one"]] {
        assert!(Command::new("git")
            .args(args)
            .current_dir(&dir)
            .status()
            .unwrap()
            .success());
    }
    dir
}

fn capture() -> (
    std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    impl FnMut(PluginEvent) -> anyhow::Result<()>,
) {
    let said = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let sink = said.clone();
    let emit = move |ev: PluginEvent| {
        if let PluginEvent::Message { text, .. } = &ev {
            sink.lock().unwrap().push(text.clone());
        }
        Ok(())
    };
    (said, emit)
}

/// Run `f` with the process CWD inside `dir` — `undo` reads the working
/// directory, which is what a broker actually has.
fn in_dir<T>(dir: &Path, f: impl FnOnce() -> T) -> T {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let was = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir).unwrap();
    let out = f();
    std::env::set_current_dir(was).unwrap();
    out
}

#[test]
fn an_undo_offer_says_what_would_come_back_and_changes_nothing_yet() {
    let dir = repo("offer");
    let session = Session::default();
    let (said, mut emit) = capture();
    in_dir(&dir, || {
        super::super::checkpoint::snapshot(&dir, "before the task").unwrap();
        std::fs::write(dir.join("a.txt"), "after\n").unwrap();
        offer(&session, 0, &mut emit).unwrap();
    });
    let text = said.lock().unwrap().join("\n");
    assert!(text.contains("would put back"), "{text}");
    assert!(text.contains("a.txt"), "{text}");
    assert!(pending(&session), "the offer was not held");
    assert_eq!(
        std::fs::read_to_string(dir.join("a.txt")).unwrap(),
        "after\n",
        "an OFFER wrote to the tree"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_confirm_word_puts_the_files_back_and_the_reject_word_leaves_them() {
    let dir = repo("apply");
    let session = Session::default();
    let (said, mut emit) = capture();
    in_dir(&dir, || {
        super::super::checkpoint::snapshot(&dir, "before the task").unwrap();
        std::fs::write(dir.join("a.txt"), "after\n").unwrap();
        gate("undo that", &session, &mut emit)
            .expect("undo was not recognised")
            .unwrap();
        gate("no", &session, &mut emit)
            .expect("the reject word was not a gate")
            .unwrap();
        assert!(!pending(&session), "a rejected offer stayed pending");
        assert_eq!(
            std::fs::read_to_string(dir.join("a.txt")).unwrap(),
            "after\n"
        );
        gate("undo that", &session, &mut emit).unwrap().unwrap();
        gate("yes", &session, &mut emit)
            .expect("the confirm word was not a gate")
            .unwrap();
    });
    assert_eq!(
        std::fs::read_to_string(dir.join("a.txt")).unwrap(),
        "before\n",
        "the file did not come back"
    );
    let text = said.lock().unwrap().join("\n");
    assert!(text.contains("undone"), "{text}");
    assert!(text.contains("left as it is"), "{text}");
    assert!(!pending(&session));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_tree_that_already_matches_the_checkpoint_has_nothing_to_undo() {
    let dir = repo("clean");
    let session = Session::default();
    let (said, mut emit) = capture();
    in_dir(&dir, || {
        super::super::checkpoint::snapshot(&dir, "before the task").unwrap();
        offer(&session, 0, &mut emit).unwrap();
    });
    assert!(said.lock().unwrap().join("\n").contains("nothing to undo"));
    assert!(!pending(&session), "an empty offer was held anyway");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn an_ordinary_task_is_not_a_gate_even_with_an_offer_held() {
    let session = Session::default();
    let (_said, mut emit) = capture();
    *lock(&session.undo) = Some(Pending {
        sha: "deadbeef".into(),
        label: "before the task".into(),
        files: vec!["a.txt".into()],
    });
    assert!(
        gate("add a test for the router", &session, &mut emit).is_none(),
        "a task was swallowed by the undo gate"
    );
    assert!(pending(&session), "the offer should still be waiting");
}

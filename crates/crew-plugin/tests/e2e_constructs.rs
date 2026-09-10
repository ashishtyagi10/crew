//! Every construct the router advertises must actually answer.
//!
//! Thirty-odd releases in one run reshaped this command surface heavily —
//! constructs deleted, folded into each other, and one (`/reload`) found to
//! have been listed for eleven releases with no palette row. The lists are
//! bound to each other by unit tests now; this binds them to the RUNNING
//! broker, which is the only thing that can prove a handler still exists.
mod common;
use common::{messages, run_broker, seed_specialists, unique_dir};
use crew_plugin::PluginEvent;

/// Constructs that mutate the working tree or the user's session in ways a
/// smoke test should not trigger blindly. They are exercised by their own
/// tests; here they would restore files or rewrite state.
const SKIP: &[&str] = &["restore"];

#[test]
fn every_advertised_construct_answers() {
    let dir = unique_dir("constructs-smoke");
    seed_specialists(&dir, &["planner"]);
    let mock = ("CREW_BROKER_MOCK_REPLY", "ok\n@done");

    let names: Vec<&str> = crew_plugin::broker_constructs()
        .iter()
        .copied()
        .filter(|c| !SKIP.contains(c))
        .collect();
    // The command diet pinned the surface at seven (retire_tests owns the
    // exact list); minus the skipped `/restore` that leaves six to smoke.
    assert!(names.len() >= 6, "suspiciously few constructs: {names:?}");

    let sends: Vec<String> = names
        .iter()
        .map(|c| format!(r#"{{"type":"send","channel":"crew","text":"/{c}"}}"#))
        .collect();
    let lines: Vec<&str> = sends.iter().map(String::as_str).collect();
    let events = run_broker(&dir, &[mock], &lines);
    let msgs = messages(&events);
    // Bare `/logout` answers with a picker EVENT, not a message: the rows
    // to choose from. That is an answer too.
    let pickers = events
        .iter()
        .filter(|e| matches!(e, PluginEvent::SignOut { .. }))
        .count();

    // "unknown construct" is the one answer that means a handler is missing.
    // A usage line is a perfectly good reply to a construct given no argument.
    let unknown: Vec<&(String, String)> = msgs
        .iter()
        .filter(|(_, t)| t.contains("unknown construct"))
        .collect();
    assert!(
        unknown.is_empty(),
        "constructs the router advertises but cannot answer: {unknown:?}"
    );
    // …and something came back at all, so an empty run cannot pass silently.
    assert!(
        msgs.len() + pickers >= names.len(),
        "only {} replies + {pickers} pickers for {} constructs: {msgs:?}",
        msgs.len(),
        names.len()
    );
}

/// `/diff` answers with the PATCH, in the fence the pane's diff lexer renders
/// — not the stat alone — through the real binary, against a real repository
/// with one edited file. The unit tests own the wording; this proves the
/// running broker hands the pane something it will draw as a diff.
#[test]
fn diff_answers_with_a_fenced_patch() {
    let dir = unique_dir("constructs-diff");
    seed_specialists(&dir, &["planner"]);
    for args in [
        &["init", "-q"][..],
        &["config", "user.email", "t@t"],
        &["config", "user.name", "t"],
    ] {
        assert!(std::process::Command::new("git")
            .args(args)
            .current_dir(&dir)
            .status()
            .unwrap()
            .success());
    }
    std::fs::write(dir.join("f.txt"), "one\n").unwrap();
    for args in [&["add", "-A"][..], &["commit", "-q", "-m", "seed"]] {
        assert!(std::process::Command::new("git")
            .args(args)
            .current_dir(&dir)
            .status()
            .unwrap()
            .success());
    }
    std::fs::write(dir.join("f.txt"), "two\n").unwrap();

    let mock = ("CREW_BROKER_MOCK_REPLY", "ok\n@done");
    // `git` on PATH, or the comparison cannot run (see e2e_tasks).
    let path = ("PATH", "/usr/bin:/bin");
    let send = r#"{"type":"send","channel":"crew","text":"/diff"}"#;
    let msgs = messages(&run_broker(&dir, &[mock, path], &[send]));
    let reply = msgs
        .iter()
        .find(|(_, t)| t.contains("f.txt"))
        .unwrap_or_else(|| panic!("no /diff reply names the file: {msgs:?}"));
    assert!(reply.1.contains("```diff\n"), "no fence: {}", reply.1);
    assert!(reply.1.contains("@@ -1 +1 @@"), "no hunk: {}", reply.1);
    assert!(reply.1.contains("-one\n+two"), "{}", reply.1);
}

//! Every construct the router advertises must actually answer.
//!
//! Thirty-odd releases in one run reshaped this command surface heavily —
//! constructs deleted, folded into each other, and one (`/reload`) found to
//! have been listed for eleven releases with no palette row. The lists are
//! bound to each other by unit tests now; this binds them to the RUNNING
//! broker, which is the only thing that can prove a handler still exists.
mod common;
use common::{messages, run_broker, seed_specialists, unique_dir};

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
    let msgs = messages(&run_broker(&dir, &[mock], &lines));

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
        msgs.len() >= names.len(),
        "only {} replies for {} constructs: {msgs:?}",
        msgs.len(),
        names.len()
    );
}

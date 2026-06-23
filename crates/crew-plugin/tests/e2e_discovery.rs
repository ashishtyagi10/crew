//! End-to-end discovery and addressing, through the real `crew-broker-plugin`
//! binary. Discovery is real: the broker probes PATH for installed agent CLIs,
//! so we control what it finds by what fakes we drop into the PATH dir.
//!
//! (The per-call timeout is proven with a real process by the `run.rs` unit
//! test `run_cli_times_out_on_hang`, so it is not re-run slowly here.)
mod common;
use common::{has_leg, messages, run_broker, unique_dir, write_fake};

const HELLO: &str = r#"{"type":"hello","v":1}"#;

/// The roster line is the first `crew`-sender message emitted on hello.
fn roster(events: &[common::PluginEvent]) -> String {
    messages(events)
        .into_iter()
        .find(|(s, _)| s == "crew")
        .map(|(_, t)| t)
        .unwrap_or_default()
}

#[test]
fn discovery_lists_all_three() {
    let dir = unique_dir("disc3");
    for a in ["claude", "codex", "opencode"] {
        write_fake(&dir, a, &["DONE"], a == "opencode");
    }
    let r = roster(&run_broker(&dir, &[], &[HELLO]));
    assert!(r.contains("3 agent(s)"), "{r}");
    assert!(r.contains("claude") && r.contains("codex") && r.contains("opencode"));
}

#[test]
fn discovery_lists_only_installed() {
    let dir = unique_dir("disc1");
    write_fake(&dir, "claude", &["DONE"], false); // codex/opencode absent
    let r = roster(&run_broker(&dir, &[], &[HELLO]));
    assert!(r.contains("1 agent(s)"), "{r}");
    assert!(r.contains("claude"));
    assert!(!r.contains("codex"), "{r}");
}

#[test]
fn discovery_reports_none_found() {
    let dir = unique_dir("disc0"); // empty PATH dir, no fakes
    let r = roster(&run_broker(&dir, &[], &[HELLO]));
    assert!(r.contains("No coding agents"), "{r}");
}

#[test]
fn no_agents_does_not_route() {
    let dir = unique_dir("none-route");
    let send = r#"{"type":"send","channel":"crew","text":"do it"}"#;
    let ev = run_broker(&dir, &[], &[send]);
    // Only the "no agents" explanation; no relay legs.
    let msgs = messages(&ev);
    assert!(msgs.iter().all(|(s, _)| s == "crew"), "{msgs:?}");
    assert!(msgs.iter().any(|(_, t)| t.contains("No coding agents")));
}

#[test]
fn at_selector_starts_with_chosen_agent() {
    let dir = unique_dir("sel");
    write_fake(&dir, "claude", &["DONE: claude-start"], false);
    write_fake(&dir, "codex", &["DONE: codex-start"], false);
    let send = r#"{"type":"send","channel":"crew","text":"@codex hello there"}"#;
    let ev = run_broker(&dir, &[], &[send]);
    // codex (not the default first agent, claude) handled the task.
    assert!(has_leg(&ev, "codex → user"), "{:?}", messages(&ev));
    assert!(!has_leg(&ev, "claude → user"), "{:?}", messages(&ev));
}

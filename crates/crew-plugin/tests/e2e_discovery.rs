//! End-to-end discovery and addressing, through the real `crew-broker-plugin`
//! binary. The default agents are the inbuilt API roster (planner/coder/
//! reviewer); the `CREW_BROKER_MOCK_REPLY` hook backs them with a fixed-reply
//! mock so the relay runs deterministically without a network. With no API key
//! and no mock, the broker reports that none are available.
mod common;
use common::{has_leg, messages, run_broker, unique_dir};

const HELLO: &str = r#"{"type":"hello","v":1}"#;
/// Enables the inbuilt roster offline: every agent replies with this, then `@done`.
const MOCK: (&str, &str) = ("CREW_BROKER_MOCK_REPLY", "ok\n@done");

/// The roster line is the first `crew`-sender message emitted on hello.
fn roster(events: &[common::PluginEvent]) -> String {
    messages(events)
        .into_iter()
        .find(|(s, _)| s == "crew")
        .map(|(_, t)| t)
        .unwrap_or_default()
}

#[test]
fn discovery_lists_the_inbuilt_roster() {
    let dir = unique_dir("disc");
    let r = roster(&run_broker(&dir, &[MOCK], &[HELLO]));
    assert!(r.contains("3 agent(s)"), "{r}");
    assert!(
        r.contains("planner") && r.contains("coder") && r.contains("reviewer"),
        "{r}"
    );
}

#[test]
fn discovery_reports_no_key() {
    let dir = unique_dir("disc0"); // harness clears any inherited key
    let r = roster(&run_broker(&dir, &[], &[HELLO]));
    assert!(r.contains("ANTHROPIC_API_KEY"), "{r}");
}

#[test]
fn no_key_runs_offline_stub_swarm() {
    let dir = unique_dir("none-route");
    let send = r#"{"type":"send","channel":"crew","text":"do it"}"#;
    let ev = run_broker(&dir, &[], &[send]);
    // A plain (unaddressed) message is now the default swarm. With no provider
    // key it runs the deterministic offline stub swarm — no network relay —
    // announcing a plan and closing with a "swarm done" summary.
    let msgs = messages(&ev);
    assert!(
        msgs.iter()
            .any(|(s, t)| s == "crew" && t.contains("planned")),
        "{msgs:?}"
    );
    assert!(
        msgs.iter()
            .any(|(s, t)| s == "crew" && t.contains("swarm done")),
        "{msgs:?}"
    );
}

/// A GUI/stale-terminal launch misses keys added to shell config after that
/// environment was created; the broker re-imports them from `$SHELL` (here a
/// fake shell that "has" a DashScope key the process env lacks).
#[cfg(unix)]
#[test]
fn shell_env_probe_recovers_missing_provider_key() {
    use std::os::unix::fs::PermissionsExt;
    let dir = unique_dir("shellenv");
    let fake = dir.join("fakeshell");
    std::fs::write(&fake, "#!/bin/sh\necho DASHSCOPE_API_KEY=e2e-test-key\n").unwrap();
    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
    let env = [
        ("CREW_SHELL_ENV", "1"), // re-enable the probe the harness disables
        ("SHELL", fake.to_str().unwrap()),
    ];
    let r = roster(&run_broker(&dir, &env, &[HELLO]));
    assert!(r.contains("3 agent(s)"), "{r}");
}

#[test]
fn at_selector_starts_with_chosen_agent() {
    let dir = unique_dir("sel");
    let send = r#"{"type":"send","channel":"crew","text":"@reviewer hello there"}"#;
    let ev = run_broker(&dir, &[MOCK], &[send]);
    // reviewer (not the default first agent, planner) handled the task.
    assert!(has_leg(&ev, "reviewer → user"), "{:?}", messages(&ev));
    assert!(!has_leg(&ev, "planner → user"), "{:?}", messages(&ev));
}

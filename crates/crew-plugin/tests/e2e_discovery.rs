//! End-to-end discovery and addressing, through the real `crew-broker-plugin`
//! binary. There is no inbuilt agent roster any more (see
//! `broker::apiadapter::specialist_agents` doc): a fresh project has zero
//! specialists until either a run invents some (persisted to the project-local
//! store, see `broker::specialists`) or a test seeds the store directly
//! (`common::seed_specialists`). The `CREW_BROKER_MOCK_REPLY` hook backs
//! whichever agents exist with a fixed reply so the relay/swarm runs
//! deterministically without a network. With no API key and no mock, the
//! broker reports that none are available.
mod common;
use common::{has_leg, messages, run_broker, seed_specialists, unique_dir};

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

/// There is no inbuilt roster (see module doc): a fresh project's registry is
/// empty until a run invents specialists. This is the direct end-to-end
/// counterpart of `apiadapter::specialist_agents`'s doc comment — "a fresh
/// project has no specialists until a run invents some" — proven through the
/// real binary: first run a plain (unaddressed) message with the mock reply,
/// which plans through the deterministic `StubPlanner` (specialties `leaf-0`,
/// `leaf-1`, `merge`; see `swarm::backend`) and persists that cast to the
/// store before this process exits (`run_broker_stdio` joins the background
/// task at EOF, so the write has landed). Then, in a SECOND process over the
/// same project dir, `hello` rebuilds the registry from that now-populated
/// store — proving a run's invented cast IS the roster, not a probe of a
/// static trio.
#[test]
fn a_runs_invented_cast_becomes_the_roster() {
    let dir = unique_dir("disc-invent");
    let send = r#"{"type":"send","channel":"crew","text":"do it"}"#;
    run_broker(&dir, &[MOCK], &[send]);
    let r = roster(&run_broker(&dir, &[MOCK], &[HELLO]));
    assert!(r.contains("3 agent(s)"), "{r}");
    assert!(
        r.contains("leaf-0") && r.contains("leaf-1") && r.contains("merge"),
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
/// fake shell that "has" a DashScope key the process env lacks). The
/// subject under test is the key re-import, not agent count — a stored
/// specialist has no provider to run on without it, so a seeded, isolated
/// store (one specialist, `scout`) that only shows up in the roster when the
/// probe succeeds is the direct proof: without the recovered key,
/// `roster_with` finds no provider and `specialist_agents` never runs at
/// all (see its doc), leaving the roster empty regardless of the store.
#[cfg(unix)]
#[test]
fn shell_env_probe_recovers_missing_provider_key() {
    use std::os::unix::fs::PermissionsExt;
    let dir = unique_dir("shellenv");
    seed_specialists(&dir, &["scout"]);
    let fake = dir.join("fakeshell");
    std::fs::write(&fake, "#!/bin/sh\necho DASHSCOPE_API_KEY=e2e-test-key\n").unwrap();
    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
    let env = [
        ("CREW_SHELL_ENV", "1"), // re-enable the probe the harness disables
        ("SHELL", fake.to_str().unwrap()),
    ];
    let r = roster(&run_broker(&dir, &env, &[HELLO]));
    assert!(r.contains("1 agent(s)") && r.contains("scout"), "{r}");
}

/// The real subject: `@name` addressing picks who starts, rather than
/// defaulting to the first agent in the roster (`relay::split_target`'s
/// fallback). Two seeded specialists prove it — `scribe` loads first (so
/// it's what a *default* pick would choose), `reviewer` is the one actually
/// addressed. A single-specialist fixture would pass even if the selector
/// were ignored, since the default and the addressed agent would coincide.
#[test]
fn at_selector_starts_with_chosen_agent() {
    let dir = unique_dir("sel");
    seed_specialists(&dir, &["scribe", "reviewer"]);
    let send = r#"{"type":"send","channel":"crew","text":"@reviewer hello there"}"#;
    let ev = run_broker(&dir, &[MOCK], &[send]);
    // reviewer (not the default first agent, scribe) handled the task.
    assert!(has_leg(&ev, "reviewer → user"), "{:?}", messages(&ev));
    assert!(!has_leg(&ev, "scribe → user"), "{:?}", messages(&ev));
}

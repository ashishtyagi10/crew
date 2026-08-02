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
use common::{has_leg, messages, roster_names, run_broker, seed_specialists, unique_dir};

const HELLO: &str = r#"{"type":"hello","v":1}"#;
/// Enables the inbuilt roster offline: every agent replies with this, then `@done`.
const MOCK: (&str, &str) = ("CREW_BROKER_MOCK_REPLY", "ok\n@done");

/// The opening chat message `hello` emits — since the v0.6.21 splash this is
/// the Agent Smith nameplate, with the roster/key warning riding below the
/// art only on a dead-on-arrival session (no provider key). Who was actually
/// discovered lives in the structured `Roster` event (`common::roster_names`).
fn smith_text(events: &[common::PluginEvent]) -> String {
    messages(events)
        .into_iter()
        .find(|(s, _)| s == "agent smith")
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
    let names = roster_names(&run_broker(&dir, &[MOCK], &[HELLO]));
    assert_eq!(names.len(), 3, "{names:?}");
    for expected in ["leaf-0", "leaf-1", "merge"] {
        assert!(names.iter().any(|n| n == expected), "{names:?}");
    }
}

#[test]
fn discovery_reports_no_key() {
    let dir = unique_dir("disc0"); // harness clears any inherited key
    let r = smith_text(&run_broker(&dir, &[], &[HELLO]));
    // Through the one copy of the advice, not a literal of it: this assertion
    // named an env var, and env vars stopped being the only way in.
    assert!(r.contains(crew_plugin::no_provider_advice()), "{r}");
}

#[test]
fn no_key_runs_offline_stub_swarm() {
    let dir = unique_dir("none-route");
    let send = r#"{"type":"send","channel":"crew","text":"do it"}"#;
    let ev = run_broker(&dir, &[], &[send]);
    // A plain (unaddressed) message is now the default swarm. With no provider
    // key it runs the deterministic offline stub swarm — no network relay —
    // announcing a plan. A clean run closes silently (no "swarm done" chrome),
    // so the plan announcement is the proof the swarm ran.
    let msgs = messages(&ev);
    assert!(
        msgs.iter()
            .any(|(s, t)| s == "agent smith" && t.contains("planned")),
        "{msgs:?}"
    );
    assert!(
        !msgs
            .iter()
            .any(|(s, t)| s == "agent smith" && t.contains("swarm done")),
        "a clean run must close without a swarm-done summary: {msgs:?}"
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
    let names = roster_names(&run_broker(&dir, &env, &[HELLO]));
    assert_eq!(names, ["scout"], "{names:?}");
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

/// A pane opened in a project that was worked in before says so, at startup,
/// through a real broker. `/resume` has folded the previous conversation in
/// since long before this and announced itself nowhere — the same shape as
/// the checkpoint note in v0.6.77.
#[test]
fn a_previous_session_announces_itself_at_startup() {
    let dir = unique_dir("resume-offer");
    seed_specialists(&dir, &["planner"]);
    // What the LAST run left behind. `run_broker` rotates this into the
    // resumable file on start, exactly as a second launch would.
    std::fs::create_dir_all(dir.join(".crew")).unwrap();
    std::fs::write(
        dir.join(".crew").join("session-live.md"),
        "user: fix the flaky test\ncoder → user: done\n",
    )
    .unwrap();

    let msgs = messages(&run_broker(&dir, &[], &[HELLO]));
    // `/resume` retired: the offer teaches the plain-language phrasing.
    let offer = msgs
        .iter()
        .find(|(_, t)| t.contains("pick up where we left off"));
    let (_, text) = offer.unwrap_or_else(|| panic!("no resume offer: {msgs:?}"));
    assert!(text.contains("2 messages"), "{text}");
}

/// …and a first run in a fresh project says nothing at all. An offer of
/// nothing is noise on the one screen a first run is guaranteed to see.
#[test]
fn a_fresh_project_makes_no_offer() {
    let dir = unique_dir("resume-none");
    seed_specialists(&dir, &["planner"]);
    let msgs = messages(&run_broker(&dir, &[], &[HELLO]));
    // The pane DID greet — without this the assertion below passes on an
    // empty list, which is how the first draft of it passed vacuously.
    assert!(!msgs.is_empty(), "the broker never greeted at all");
    assert!(
        !msgs.iter().any(|(_, t)| t.contains("/resume")),
        "offered a session that never happened: {msgs:?}"
    );
}

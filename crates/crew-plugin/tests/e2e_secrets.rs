//! Condition 6's acceptance: after an auth round-trip and real smith
//! traffic, EVERY log sink — the session logs, the broker's stdout
//! transcript, its stderr file, `/doctor`'s output, anything under the
//! (redirected) HOME where crash.log/stderr.log would land — is grepped for
//! the token material, and the count of hits is zero. The token STORE is the
//! one file allowed to hold it.
//!
//! The grant here is seeded in the exact shape a completed device flow
//! stores (pinned by `tokens_tests`); `e2e_oauth` drives the live flow
//! through `/model` and re-runs this sweep after it.
mod common;
use common::oauthstub::sweep;
use common::{messages, run_broker, unique_dir};

const SECRET: &str = "sweep-secret-token-9000";

const DOCTOR: &str = r#"{"type":"send","channel":"crew","text":"/doctor"}"#;
const MODEL: &str = r#"{"type":"send","channel":"crew","text":"/model"}"#;
const SEND: &str = r#"{"type":"send","channel":"crew","text":"summarize the plan"}"#;

#[test]
fn no_log_sink_carries_token_material_after_traffic() {
    let dir = unique_dir("secrets-sweep");
    // A stored grant as a completed sign-in leaves it (file backend).
    std::fs::write(
        dir.join("tokens.json"),
        format!(
            r#"{{"dashscope":{{"access":"at-{SECRET}","refresh":"rt-{SECRET}","expires_at":99999999999,"resource":null}}}}"#
        ),
    )
    .unwrap();
    let home = dir.join("home");
    std::fs::create_dir_all(&home).unwrap();
    let events = run_broker(
        &dir,
        &[
            // Everything path-shaped lands inside the swept dir — including
            // where crash.log / stderr.log would go (config under HOME).
            ("HOME", home.to_str().unwrap()),
            ("CREW_BROKER_MOCK_REPLY", "the plan: ship it"),
        ],
        &[DOCTOR, MODEL, SEND],
    );
    let transcript: String = messages(&events)
        .iter()
        .map(|(s, t)| format!("{s}: {t}\n"))
        .collect();

    // The sinks are LIVE, not vacuously absent — each one carries expected
    // content before the sweep declares it clean.
    let doc = messages(&events)
        .iter()
        .find(|(_, t)| t.contains("crew doctor"))
        .map(|(_, t)| t.clone())
        .expect("doctor output is one of the swept sinks");
    assert!(
        doc.contains("token store: 0600 file (no keychain on this system)"),
        "{doc}"
    );
    let live = std::fs::read_to_string(dir.join(".crew/session-live.md")).unwrap();
    assert!(
        live.contains("the plan: ship it"),
        "session log must have recorded the traffic: {live}"
    );
    assert!(
        transcript.contains("the plan: ship it"),
        "the stdout transcript must carry the reply"
    );
    let stderr = std::fs::read_to_string(dir.join("broker-stderr.log")).unwrap_or_default();

    // THE SWEEP: zero hits outside the token store itself.
    let hits = sweep(
        &dir,
        &[("stdout-transcript", &transcript), ("stderr", &stderr)],
        SECRET,
        &["tokens.json"],
    );
    assert_eq!(hits, Vec::<String>::new(), "token material leaked");
}

/// The `/doctor` backend statement flips when a keychain answers — the same
/// run, the other backend, so the file-backend line above cannot be a
/// hardcoded sentence.
#[test]
fn doctor_states_the_keychain_backend_when_security_exists() {
    let dir = unique_dir("secrets-keychain-line");
    let msgs = messages(&run_broker(
        &dir,
        &[("CREW_SECURITY_BIN", "/usr/bin/true")],
        &[DOCTOR],
    ));
    let doc = msgs
        .iter()
        .find(|(_, t)| t.contains("crew doctor"))
        .map(|(_, t)| t.clone())
        .unwrap_or_default();
    assert!(
        doc.contains("token store: OS keychain (macOS `security`)"),
        "{doc}"
    );
}

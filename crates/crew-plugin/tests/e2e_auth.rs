//! End-to-end: the CLI-delegated subscription rung, through the real
//! broker binary with fake vendor CLIs on an isolated `PATH`. The fakes
//! answer their own status commands (`claude auth status`, `codex login
//! status`) exactly like the real ones — exit code plus a stdout marker —
//! so the probe exercises the same signal it reads in production, and no
//! real CLI, token store, key or network is ever touched.
mod common;
use common::{messages, run_broker, unique_dir};
use std::path::Path;

/// A fake vendor CLI: answers `<bin> <status_head> …` as its status command
/// (signed in ⇒ marker + exit 0, signed out ⇒ marker + exit 1) and echoes a
/// reply for anything else (the relay invocation).
fn write_cli(dir: &Path, bin: &str, status_head: &str, signed_in: bool, reply: &str) {
    let (marker, code) = if signed_in {
        ("Logged in", 0)
    } else {
        ("Not logged in", 1)
    };
    let script = format!(
        "#!/bin/sh\nif [ \"$1\" = \"{status_head}\" ]; then echo '{marker}'; exit {code}; fi\n\
         echo '{reply}'\n"
    );
    let path = dir.join(bin);
    std::fs::write(&path, script).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}

const SEND: &str = r#"{"type":"send","channel":"crew","text":"say hi"}"#;

/// The subscription rung routes through the SIGNED-IN CLI, not merely the
/// first installed one: claude is installed but signed out, codex is signed
/// in, so a plain task must open with codex — under the old keyless-relay
/// rule (first registered leads) it would have opened with claude.
#[test]
fn a_signed_in_cli_outranks_a_merely_installed_one() {
    let dir = unique_dir("auth-signed");
    write_cli(&dir, "claude", "auth", false, "claude here");
    write_cli(&dir, "codex", "login", true, "codex here");
    let msgs = messages(&run_broker(&dir, &[], &[SEND]));
    assert!(
        msgs.iter()
            .any(|(s, t)| s == "agent smith" && t.contains("starting with codex")),
        "the signed-in codex must lead the relay: {msgs:?}"
    );
    assert!(
        msgs.iter()
            .any(|(s, t)| s.starts_with("codex") && t.contains("codex here")),
        "the task must actually reach the codex CLI: {msgs:?}"
    );
}

/// With every CLI signed out, the keyless relay keeps its old shape: the
/// first registered agent (claude) leads, exactly as before the rung.
#[test]
fn all_signed_out_keeps_the_keyless_lead() {
    let dir = unique_dir("auth-out");
    write_cli(&dir, "claude", "auth", false, "claude here");
    write_cli(&dir, "codex", "login", false, "codex here");
    let msgs = messages(&run_broker(&dir, &[], &[SEND]));
    assert!(
        msgs.iter()
            .any(|(s, t)| s == "agent smith" && t.contains("starting with claude")),
        "signed-out CLIs fall back to the keyless first-registered lead: {msgs:?}"
    );
}

/// `/doctor` states the subscription on its provider line — including that
/// swarm planning degrades to the relay — and gives every provider an auth
/// line of its own.
#[test]
fn doctor_states_the_subscription_and_the_swarm_degradation() {
    let dir = unique_dir("auth-doctor");
    write_cli(&dir, "codex", "login", true, "codex here");
    let doctor = r#"{"type":"send","channel":"crew","text":"/doctor"}"#;
    let msgs = messages(&run_broker(&dir, &[], &[doctor]));
    let doc = msgs
        .iter()
        .find(|(s, t)| s == "agent smith" && t.contains("crew doctor"))
        .map(|(_, t)| t.clone())
        .unwrap_or_default();
    assert!(doc.contains("codex subscription"), "{doc}");
    assert!(doc.contains("degrades to the relay"), "{doc}");
    assert!(doc.contains("codex: signed in (subscription)"), "{doc}");
    assert!(doc.contains("claude-code: not installed"), "{doc}");
}

/// `CREW_SUBSCRIPTIONS=0` switches the rung off: signed-in state is never
/// probed and the old keyless lead applies.
#[test]
fn the_kill_switch_disables_the_subscription_rung() {
    let dir = unique_dir("auth-off");
    write_cli(&dir, "claude", "auth", false, "claude here");
    write_cli(&dir, "codex", "login", true, "codex here");
    let msgs = messages(&run_broker(&dir, &[("CREW_SUBSCRIPTIONS", "0")], &[SEND]));
    assert!(
        msgs.iter()
            .any(|(s, t)| s == "agent smith" && t.contains("starting with claude")),
        "with the rung off, the first registered agent leads: {msgs:?}"
    );
}

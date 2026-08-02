//! End-to-end test harness: drive the *real* `crew-broker-plugin` binary,
//! feeding it JSON commands and parsing the events it streams back. Agent
//! replies are made deterministic (no network) via `CREW_BROKER_MOCK_REPLY`;
//! `write_fake` still builds fake CLI agents on `PATH` for tests of that path.
//! There is no inbuilt agent roster (see `broker::apiadapter::specialist_agents`
//! doc): the agents a run can address come from the project-local specialist
//! store (`broker::specialists`), so `run_broker` points `CREW_PROJECT_DIR` at
//! a fresh, per-test directory — the seam `broker::specialists::base_dir`
//! already exposes for exactly this. Without it every e2e process shares the
//! developer's real `./.crew/specialists.json` (relative to the crate's CWD
//! under `cargo test`): tests pollute the working tree and each other's
//! results depending on run order. Reusing `path_dir` (already unique per
//! test, via `unique_dir`) as the project dir too, instead of allocating a
//! second directory, keeps one directory per test standing for "this test's
//! isolated environment" rather than splitting it across two — fake agents
//! (if any) and the specialist store never collide because they're
//! different filenames.
//!
//! Each e2e file includes this module separately and uses a different subset of
//! the helpers, so unused-in-one-file helpers are expected.
#![allow(dead_code)]
pub mod oauthstub;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};

pub use crew_plugin::PluginEvent;

static SEQ: AtomicU32 = AtomicU32::new(0);

/// A fresh, isolated temp directory used both as the fake-agent `PATH` and as
/// the home for each fake's call-counter file.
pub fn unique_dir(tag: &str) -> PathBuf {
    let id = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("crew-e2e-{tag}-{}-{id}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Write an executable fake agent named `name` into `dir`. On the Nth call it
/// prints the Nth entry of `replies` (then `DONE`), using only shell builtins
/// so it works with `PATH` restricted to `dir`. `json` wraps the reply in an
/// opencode-style event line to exercise the JSON normalizer.
pub fn write_fake(dir: &Path, name: &str, replies: &[&str], json: bool) {
    let cnt = dir.join(format!("{name}.cnt"));
    let arms: String = replies
        .iter()
        .enumerate()
        .map(|(i, r)| format!("{i}) R='{r}' ;;\n"))
        .collect();
    // `%b` interprets `\n` in a reply as a real newline (so directives land on
    // their own line). For JSON, the literal `\n` stays a valid JSON string
    // escape that serde decodes back to a newline.
    let emit = if json {
        r#"printf '{"type":"text","text":"%s"}\n' "$R""#
    } else {
        r#"printf '%b\n' "$R""#
    };
    let script = format!(
        "#!/bin/sh\nCNT='{cnt}'\nn=0\n[ -f \"$CNT\" ] && read n < \"$CNT\"\n\
         echo $((n+1)) > \"$CNT\"\ncase \"$n\" in\n{arms}*) R='DONE' ;;\nesac\n{emit}\n",
        cnt = cnt.display(),
    );
    let path = dir.join(name);
    std::fs::write(&path, script).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}

/// Run the real broker binary with `PATH` set to `path_dir` (so only the fakes
/// there are discoverable), feeding `cmds` as stdin JSON lines and returning the
/// parsed events it emitted.
pub fn run_broker(path_dir: &Path, env: &[(&str, &str)], cmds: &[&str]) -> Vec<PluginEvent> {
    let bin = env!("CARGO_BIN_EXE_crew-broker-plugin");
    let mut command = Command::new(bin);
    configure(&mut command, path_dir, env);
    let mut child = command.spawn().unwrap();
    {
        let mut stdin = child.stdin.take().unwrap();
        for line in cmds {
            writeln!(stdin, "{line}").unwrap();
        }
    } // drop stdin → EOF → the broker's read loop ends and it exits
    let out = child.wait_with_output().unwrap();
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| serde_json::from_str::<PluginEvent>(l).ok())
        .collect()
}

/// The isolation every broker child gets (one copy for both runners).
fn configure(command: &mut Command, path_dir: &Path, env: &[(&str, &str)]) {
    command
        .env("PATH", path_dir)
        // Isolate the project-local specialist store (see module doc): each
        // test gets its own empty `<path_dir>/.crew/specialists.json` instead
        // of sharing (and mutating) the crate's real working-tree store.
        .env("CREW_PROJECT_DIR", path_dir)
        // Belt-and-braces alongside `CREW_PROJECT_DIR`: `broker::sessionlog`
        // (the auto-saved `.crew/session-live.md` / `last-session.md`) has no
        // env override and always resolves against the process CWD, so give
        // the broker its own CWD too — otherwise every e2e run would still
        // scribble session logs into the crate's real working tree.
        .current_dir(path_dir)
        // Determinism: never let an inherited API key make the broker reach the
        // real network during tests. Tests opt into agents via the mock hook.
        //
        // From `credentials::VARS`, not a list. This was the third hand-kept
        // copy of "the provider keys" — the unit guard was the second — and a
        // list that names three of six silently stops isolating the moment a
        // developer exports one of the others.
        .env_remove("CREW_BROKER_MOCK_REPLY")
        // …and never let the shell-env probe re-import those keys from the
        // developer's real shell config. Tests opt in via the `env` pairs.
        .env("CREW_SHELL_ENV", "0")
        // Clearing the environment stopped being enough once keys could be
        // supplied from inside crew (v0.6.35 key popup, v0.6.37 OpenRouter
        // sign-in): the credential store is a real-disk singleton under
        // `dirs::config_dir()`, so a developer who has signed in makes every
        // "no provider" test resolve a provider anyway — silently, since the
        // store is read through `key_for` and never mentions itself. Point it
        // at a path inside this test's own dir, which is exactly what
        // `credentials::path`'s `CREW_CREDENTIALS_PATH` override exists for.
        // A test that WANTS a stored key writes that file and passes its own
        // pair in `env`.
        .env("CREW_CREDENTIALS_PATH", path_dir.join("credentials.json"))
        // Keychain hermeticity: never let a broker child under test reach the
        // developer's real macOS keychain. Empty = "no keychain here" (the
        // file backend); a test that wants one passes a fake via `env`.
        .env("CREW_SECURITY_BIN", "")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        // To a file, not /dev/null: stderr is one of the log sinks the
        // secret-sweep test greps, and a nulled stream can hide a leak.
        .stderr(std::fs::File::create(path_dir.join("broker-stderr.log")).unwrap());
    for var in crew_plugin::credentials::VARS {
        command.env_remove(var);
    }
    for (k, v) in env {
        command.env(k, v);
    }
}

/// [`run_broker`] with a pause after each command, for flows whose ORDER is
/// the point — a device sign-in must complete (its poll loop sleeping real
/// seconds against the stub server) before the next message relies on the
/// grant it stored. `cmds` pairs each JSON line with the milliseconds to
/// wait after sending it.
pub fn run_broker_paced(
    path_dir: &Path,
    env: &[(&str, &str)],
    cmds: &[(&str, u64)],
) -> Vec<PluginEvent> {
    let bin = env!("CARGO_BIN_EXE_crew-broker-plugin");
    let mut command = Command::new(bin);
    configure(&mut command, path_dir, env);
    let mut child = command.spawn().unwrap();
    {
        let mut stdin = child.stdin.take().unwrap();
        for (line, wait_ms) in cmds {
            writeln!(stdin, "{line}").unwrap();
            stdin.flush().unwrap();
            std::thread::sleep(std::time::Duration::from_millis(*wait_ms));
        }
    }
    let out = child.wait_with_output().unwrap();
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| serde_json::from_str::<PluginEvent>(l).ok())
        .collect()
}

/// Flatten events into `(sender, text)` message pairs for assertions.
pub fn messages(events: &[PluginEvent]) -> Vec<(String, String)> {
    events
        .iter()
        .filter_map(|e| match e {
            PluginEvent::Message { sender, text, .. } => Some((sender.clone(), text.clone())),
            _ => None,
        })
        .collect()
}

/// Agent names from the structured `Roster` event `hello` emits — the
/// machine-readable roster. Since the v0.6.21 splash, `hello`'s chat message
/// is the Agent Smith nameplate alone (no roster text), so tests that care
/// about WHO was discovered must read this event, not the message stream.
pub fn roster_names(events: &[PluginEvent]) -> Vec<String> {
    events
        .iter()
        .find_map(|e| match e {
            PluginEvent::Roster { agents } => Some(agents.iter().map(|a| a.name.clone()).collect()),
            _ => None,
        })
        .unwrap_or_default()
}

/// True if any message has exactly this sender label (e.g. `"claude → codex"`).
pub fn has_leg(events: &[PluginEvent], sender: &str) -> bool {
    messages(events).iter().any(|(s, _)| s == sender)
}

/// Seed `dir`'s isolated specialist store (`CREW_PROJECT_DIR`, see module doc)
/// with a fixed cast, so a test can address a known `@name` — or prove a
/// selector picks something other than the first entry — without waiting for
/// a run to invent one. `names` are written in order, earliest first, which is
/// the order `broker::specialists::load_at` (and so the roster/default-agent
/// fallback) returns them in. The JSON shape mirrors
/// `broker::specialists::Specialist` (`name`, `role`, `last_used`); `role` is
/// left blank and `last_used` is just a distinct, increasing stand-in — none
/// of these fixtures exercise LRU eviction.
pub fn seed_specialists(dir: &Path, names: &[&str]) {
    let store = dir.join(".crew").join("specialists.json");
    std::fs::create_dir_all(store.parent().unwrap()).unwrap();
    let entries: Vec<serde_json::Value> = names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            serde_json::json!({
                "name": name,
                "role": "",
                "last_used": (i + 1) as u64,
            })
        })
        .collect();
    std::fs::write(&store, serde_json::to_string_pretty(&entries).unwrap()).unwrap();
}

/// A tripwire on the SHAPE of the list this harness clears, not on the
/// clearing itself — the clearing is proved by `discovery_reports_no_key`,
/// which fails outright if any provider key reaches the child.
///
/// Worth having anyway: it fires if a credential variable stops looking like
/// one, which is the change most likely to slip past a harness that now
/// derives its list instead of naming it.
#[test]
fn credential_variables_look_like_credential_variables() {
    for var in crew_plugin::credentials::VARS {
        assert!(
            var.ends_with("_API_KEY"),
            "{var} does not look like a provider key; check `run_broker` still \
             clears everything it should"
        );
    }
}

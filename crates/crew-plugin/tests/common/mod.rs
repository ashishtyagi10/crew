//! End-to-end test harness: drive the *real* `crew-broker-plugin` binary with
//! fake agent CLIs on `PATH`, so the whole pipeline runs with real processes
//! (discovery, the JSON plugin protocol, subprocess spawning, normalization,
//! routing) but with scripted, deterministic replies.
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
    let emit = if json {
        r#"printf '{"type":"text","text":"%s"}\n' "$R""#
    } else {
        r#"printf '%s\n' "$R""#
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
    command
        .env("PATH", path_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    for (k, v) in env {
        command.env(k, v);
    }
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

/// True if any message has exactly this sender label (e.g. `"claude → codex"`).
pub fn has_leg(events: &[PluginEvent], sender: &str) -> bool {
    messages(events).iter().any(|(s, _)| s == sender)
}

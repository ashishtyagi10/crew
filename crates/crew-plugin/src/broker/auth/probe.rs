//! Signed-in probes for CLI-delegated providers — consent-based only. The
//! probe runs the vendor CLI's OWN status command (`claude auth status`,
//! `codex login status`) off the pane thread with a hard timeout and reads
//! its verdict from the exit code and output markers. It NEVER opens
//! another app's token store; the CLI is the sole authority on its login.
use std::collections::HashMap;
use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use super::registry::CliSpec;
use crew_hive::childproc::no_console_window;

/// One CLI's reported auth state.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum CliAuth {
    SignedIn,
    SignedOut,
    /// The binary is not installed at all.
    Absent,
    /// Installed, but the status command failed to answer (unknown
    /// subcommand on an old CLI version, timeout, spawn failure). Treated
    /// as "no subscription" so resolution stays byte-compatible.
    Unknown,
}

/// What one run of a status command means.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Verdict {
    SignedIn,
    SignedOut,
    /// The CLI didn't recognize the subcommand — no verdict at all.
    NotACommand,
}

/// Read a status command's result. The exit code is the primary signal
/// (both `claude auth status` and `codex login status` exit 0 exactly when
/// signed in); output markers guard the two ways an exit code can lie —
/// a CLI that prints `"loggedIn": false` politely (exit 0), and one that
/// fails because the subcommand doesn't exist rather than because the user
/// is signed out. A spec with a `signed_in_marker` is the third way: its
/// exit code says nothing (`ant auth status` reports, never fails), so the
/// marker alone decides signed-in and the not-a-command markers still guard.
fn classify(ok: bool, output: &str, marker: Option<&str>) -> Verdict {
    let low = output.to_ascii_lowercase();
    if let Some(m) = marker {
        if low.contains(&m.to_ascii_lowercase()) {
            return Verdict::SignedIn;
        }
        return if not_a_command(&low) {
            Verdict::NotACommand
        } else {
            Verdict::SignedOut
        };
    }
    let signed_out = [
        "not logged in",
        "logged out",
        "not authenticated",
        "\"loggedin\": false",
        "\"loggedin\":false",
    ]
    .iter()
    .any(|m| low.contains(m));
    if signed_out {
        return Verdict::SignedOut;
    }
    if ok {
        return Verdict::SignedIn;
    }
    if not_a_command(&low) {
        Verdict::NotACommand
    } else {
        Verdict::SignedOut
    }
}

/// The CLI didn't recognize the subcommand at all (`low` is lowercased).
fn not_a_command(low: &str) -> bool {
    ["unknown command", "unrecognized", "usage:"]
        .iter()
        .any(|m| low.contains(m))
}

/// A status-command runner: `(bin, args)` → `Some((exit-ok, output))`, or
/// `None` on spawn failure / timeout.
pub(crate) type StatusRunner<'a> = &'a dyn Fn(&str, &[&str]) -> Option<(bool, String)>;

/// Probe one CLI through an injected runner (`None` = spawn failure or
/// timeout), so every state is table-testable without touching `$PATH`.
pub(crate) fn probe_with(spec: &CliSpec, installed: bool, run: StatusRunner) -> CliAuth {
    if !installed {
        return CliAuth::Absent;
    }
    match run(spec.bin, spec.status) {
        None => CliAuth::Unknown,
        Some((ok, out)) => match classify(ok, &out, spec.signed_in_marker) {
            Verdict::SignedIn => CliAuth::SignedIn,
            Verdict::SignedOut => CliAuth::SignedOut,
            Verdict::NotACommand => CliAuth::Unknown,
        },
    }
}

/// Hard ceiling on one status probe. Generous for a CLI that phones home to
/// validate, tiny next to the model calls this thread exists to make.
const PROBE_TIMEOUT: Duration = Duration::from_secs(5);

/// Run `bin args…` to completion, returning (exit-ok, stdout, stderr) — or
/// `None` on spawn failure or timeout (the child is killed). Unlike
/// `run::run_cli` this keeps the exit STATUS, which is the probe's primary
/// signal; empty output is fine here, not a failure. The streams stay
/// SEPARATE because `keychain` reads a secret from stdout, where a stderr
/// warning merged in would corrupt it.
pub(crate) fn run_split(
    bin: &str,
    args: &[&str],
    timeout: Duration,
) -> Option<(bool, String, String)> {
    let mut child = no_console_window(&mut Command::new(bin))
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;
    // Drain both pipes on side threads so a chatty CLI can't deadlock
    // against an unread pipe while we poll the deadline (run.rs's pattern).
    let drain = |r: Option<Box<dyn Read + Send>>| {
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        if let Some(mut r) = r {
            std::thread::spawn(move || {
                let mut s = String::new();
                let _ = r.read_to_string(&mut s);
                let _ = tx.send(s);
            });
        }
        rx
    };
    let out_rx = drain(child.stdout.take().map(|s| Box::new(s) as _));
    let err_rx = drain(child.stderr.take().map(|s| Box::new(s) as _));
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let out = out_rx.recv_timeout(timeout).unwrap_or_default();
                let err = err_rx.recv_timeout(timeout).unwrap_or_default();
                return Some((status.success(), out, err));
            }
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(25)),
            Err(_) => return None,
        }
    }
}

/// [`run_split`] with the streams merged — the probe's `classify` reads its
/// markers from either.
pub(crate) fn run_status(bin: &str, args: &[&str], timeout: Duration) -> Option<(bool, String)> {
    run_split(bin, args, timeout).map(|(ok, mut out, err)| {
        out.push_str(&err);
        (ok, out)
    })
}

/// [`probe_with`] over the real machine: PATH presence, then the CLI's own
/// status command under [`PROBE_TIMEOUT`].
fn live(spec: &CliSpec) -> CliAuth {
    probe_with(spec, super::super::run::on_path(spec.bin), &|b, a| {
        run_status(b, a, PROBE_TIMEOUT)
    })
}

/// Whether subscription probing is enabled. On by default;
/// `CREW_SUBSCRIPTIONS=0` switches the rung off entirely (no processes are
/// spawned and every CLI reads as state-unknown, i.e. today's behavior).
fn enabled() -> bool {
    std::env::var("CREW_SUBSCRIPTIONS").map_or(true, |v| v != "0")
}

/// How long a NEGATIVE verdict (signed out, absent, unknown) is trusted
/// before the CLI is asked again. A user who runs `ant auth login` or
/// `brew install …` in a terminal beside the pane used to need a NEW pane
/// for crew to notice (one probe per broker process, for its whole life);
/// now the next message after this window re-probes. A signed-in verdict
/// holds for the process: the mint is what notices a sign-out (a failed
/// mint arms the re-auth prompt), and a delegated CLI reports its own.
pub(crate) const NEGATIVE_TTL: Duration = Duration::from_secs(30);

/// Whether a cached verdict of `age` still stands. Pure.
pub(crate) fn still_fresh(v: CliAuth, age: Duration) -> bool {
    v == CliAuth::SignedIn || age < NEGATIVE_TTL
}

type Cache = Mutex<HashMap<&'static str, (CliAuth, Instant)>>;

fn cache() -> &'static Cache {
    static CACHE: OnceLock<Cache> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// [`live`], cached per broker process per CLI: a sign-in for the process's
/// life, anything else for [`NEGATIVE_TTL`]. Off (`CREW_SUBSCRIPTIONS=0`)
/// nothing is spawned and every CLI reads unknown.
pub(crate) fn state_cached(spec: &CliSpec) -> CliAuth {
    if !enabled() {
        return CliAuth::Unknown;
    }
    let mut m = cache().lock().unwrap_or_else(|e| e.into_inner());
    if let Some((v, at)) = m.get(spec.bin) {
        if still_fresh(*v, at.elapsed()) {
            return *v;
        }
    }
    let v = live(spec);
    m.insert(spec.bin, (v, Instant::now()));
    v
}

/// [`live`] right now, and the cache learns the answer — for the moments a
/// user explicitly asks (`/model <provider>`, `/logout`): they may have just installed
/// or signed in to the CLI in a terminal, and a listing that repeats a
/// stale "not installed" reads as crew not working.
pub(crate) fn state_fresh(spec: &CliSpec) -> CliAuth {
    if !enabled() {
        return CliAuth::Unknown;
    }
    let v = live(spec);
    let mut m = cache().lock().unwrap_or_else(|e| e.into_inner());
    m.insert(spec.bin, (v, Instant::now()));
    v
}

#[cfg(test)]
#[path = "probe_tests.rs"]
mod tests;

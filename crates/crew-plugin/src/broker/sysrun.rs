//! `sys:run` — one shell command, bounded: `/bin/sh -c`, null stdin, both
//! pipes drained on threads (capped at [`super::systools::CAP`], but always
//! drained so a chatty child never blocks on a full pipe), child killed at
//! the deadline. The broker is a subprocess, so blocking here never touches
//! the app's winit thread; the timeout bounds the hop.
use std::io::Read;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use super::systools::CAP;

/// Per-command deadline. `CREW_SYS_TIMEOUT_MS` overrides (default 30 s).
fn timeout() -> Duration {
    let ms = std::env::var("CREW_SYS_TIMEOUT_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30_000);
    Duration::from_millis(ms)
}

pub(crate) fn run(cmd: &str) -> Result<String, String> {
    run_with(cmd, timeout())
}

pub(crate) fn run_with(cmd: &str, timeout: Duration) -> Result<String, String> {
    let mut child = Command::new("/bin/sh")
        .args(["-c", cmd])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // Put `sh` in its own new process group (pgid == its own pid). A command
        // can background a descendant (`sleep 5 &`, `nohup … &`, disown) that
        // inherits our stdout/stderr pipe write-ends; `sh` then exits while that
        // descendant keeps running and keeps the pipe open. `try_wait()` only
        // observes the direct `sh` child, so without a group we'd have no way to
        // reach the orphaned descendant to stop it from wedging the drain
        // threads' `read()` (which only returns on EOF, i.e. every writer
        // closing the pipe). Owning the group lets us kill the whole subtree at
        // once, by pgid, once `sh` is done with it.
        .process_group(0)
        .spawn()
        .map_err(|e| format!("spawn /bin/sh: {e}"))?;
    let pgid = child.id();

    // Each drain thread sends its result over a channel instead of being
    // joined directly: a join() blocks unboundedly on the thread finishing,
    // which only happens once its pipe end sees EOF. If a backgrounded
    // descendant is still holding the pipe open, that may never happen. A
    // channel lets the receiver bound the wait with `recv_timeout` instead.
    let (out_tx, out_rx) = mpsc::channel();
    let (err_tx, err_rx) = mpsc::channel();
    drain(child.stdout.take().expect("piped"), out_tx);
    drain(child.stderr.take().expect("piped"), err_tx);

    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                // Direct kill only reaches `sh`; sweep the whole group so any
                // backgrounded descendant is stopped too (best effort — we're
                // already returning an error and don't wait on the drain
                // threads here, so this is purely process hygiene).
                kill_group(pgid);
                return Err(format!("timed out after {timeout:?}; command killed"));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => return Err(format!("wait: {e}")),
        }
    };

    // `sh` exiting doesn't mean the group is empty — a backgrounded
    // descendant can outlive it. Kill the group now so any such descendant
    // releases the pipe, then bound how long we wait for each drain thread's
    // result instead of joining unconditionally. The wait is capped at a
    // small grace (<=500ms) regardless of how much of the deadline remains,
    // since by this point the group kill should make the pipes close almost
    // immediately in the normal case; if a result still doesn't show up
    // (e.g. an unkillable/zombie descendant), proceed with what we have
    // rather than block.
    kill_group(pgid);
    let grace = deadline
        .saturating_duration_since(Instant::now())
        .min(Duration::from_millis(500));
    let (stdout, out_cut) = out_rx.recv_timeout(grace).unwrap_or_default();
    let (stderr, err_cut) = err_rx.recv_timeout(grace).unwrap_or_default();
    let mut text = format!("exit {}\n{stdout}", status.code().unwrap_or(-1));
    if !stderr.is_empty() {
        text.push_str(&format!("\n--- stderr ---\n{stderr}"));
    }
    if out_cut || err_cut {
        text.push_str("\n\u{2026} (output truncated at 64 KB)");
    }
    Ok(text)
}

/// Best-effort termination of a whole process group (`sh` and anything it
/// backgrounded). No `libc`/`nix` dependency is available here, so this
/// shells out to `/bin/kill -<signal> -<pgid>`, which is the standard way to
/// signal a group by pid on unix. `sh`'s pid is the group's pgid because the
/// group was created via `process_group(0)` at spawn time. TERM first to let
/// well-behaved processes clean up, then KILL to guarantee they're gone;
/// both are best-effort since the group may already be empty.
fn kill_group(pgid: u32) {
    let target = format!("-{pgid}");
    // An already-empty group makes `/bin/kill` print "No such process" — that
    // is expected (the command finished with no survivors), so silence its
    // stderr rather than leak it to the broker's output.
    for sig in ["-TERM", "-KILL"] {
        let _ = Command::new("/bin/kill")
            .args([sig, "--", &target])
            .stderr(Stdio::null())
            .status();
        if sig == "-TERM" {
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

/// Drain a pipe fully on a thread, keeping at most [`CAP`] bytes, and send
/// the kept text (lossy UTF-8) plus whether anything was dropped back over
/// `tx`. Using a channel (rather than returning a `JoinHandle` to `join()`)
/// lets the caller bound how long it waits for this thread instead of
/// blocking until the pipe sees EOF, which a still-running backgrounded
/// descendant holding the write end could delay indefinitely.
fn drain(mut pipe: impl Read + Send + 'static, tx: mpsc::Sender<(String, bool)>) {
    std::thread::spawn(move || {
        let mut kept = Vec::new();
        let mut buf = [0u8; 8192];
        let mut cut = false;
        while let Ok(n) = pipe.read(&mut buf) {
            if n == 0 {
                break;
            }
            if kept.len() < CAP {
                let take = n.min(CAP - kept.len());
                kept.extend_from_slice(&buf[..take]);
                cut |= take < n;
            } else {
                cut = true;
            }
        }
        let _ = tx.send((String::from_utf8_lossy(&kept).into_owned(), cut));
    });
}

#[cfg(test)]
#[path = "sysrun_tests.rs"]
mod tests;

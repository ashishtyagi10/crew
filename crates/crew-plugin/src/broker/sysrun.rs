//! `sys:run` — one shell command, bounded: `/bin/sh -c`, null stdin, both
//! pipes drained on threads (capped at [`super::systools::CAP`], but always
//! drained so a chatty child never blocks on a full pipe), child killed at
//! the deadline. The broker is a subprocess, so blocking here never touches
//! the app's winit thread; the timeout bounds the hop.
use std::io::Read;
use std::process::{Command, Stdio};
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
        .spawn()
        .map_err(|e| format!("spawn /bin/sh: {e}"))?;
    let out = drain(child.stdout.take().expect("piped"));
    let err = drain(child.stderr.take().expect("piped"));

    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("timed out after {timeout:?}; command killed"));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => return Err(format!("wait: {e}")),
        }
    };

    let (stdout, out_cut) = out.join().unwrap_or_default();
    let (stderr, err_cut) = err.join().unwrap_or_default();
    let mut text = format!("exit {}\n{stdout}", status.code().unwrap_or(-1));
    if !stderr.is_empty() {
        text.push_str(&format!("\n--- stderr ---\n{stderr}"));
    }
    if out_cut || err_cut {
        text.push_str("\n\u{2026} (output truncated at 64 KB)");
    }
    Ok(text)
}

/// Drain a pipe fully on a thread, keeping at most [`CAP`] bytes. Returns the
/// kept text (lossy UTF-8) and whether anything was dropped.
fn drain(mut pipe: impl Read + Send + 'static) -> std::thread::JoinHandle<(String, bool)> {
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
        (String::from_utf8_lossy(&kept).into_owned(), cut)
    })
}

#[cfg(test)]
#[path = "sysrun_tests.rs"]
mod tests;

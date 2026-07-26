//! Bounded shell-subprocess spawning for [`super`]'s probes. Split out of
//! `shellprobe.rs` to keep that file's higher-level merge/fallback logic
//! short; these are pure process-plumbing details.
use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// Run `shell -ilc env`, the primary probe covering both PATH and the
/// provider vars in the common case.
pub(super) fn bounded_shell_env(shell: &str, timeout: Duration) -> Option<String> {
    bounded_shell_output(shell, &["-ilc", "env"], timeout)
}

/// Run `shell -lc 'printf %s "$PATH"'`, the fast PATH-only fallback used
/// when [`bounded_shell_env`] comes back empty. Deliberately `-lc`, not
/// `-ilc`: `-l` alone (`~/.zprofile`) is enough to recover a normal PATH.
pub(super) fn bounded_shell_path(shell: &str, timeout: Duration) -> Option<String> {
    bounded_shell_output(shell, &["-lc", "printf %s \"$PATH\""], timeout)
}

/// Spawn `shell` with `args`, killing it if it hasn't produced output
/// within `timeout` (`Command::output()` alone has no deadline); stdout is
/// drained on a side thread while this one polls for a result or deadline.
fn bounded_shell_output(shell: &str, args: &[&str], timeout: Duration) -> Option<String> {
    let mut child = Command::new(shell)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut stdout = child.stdout.take()?;
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut s = String::new();
        let _ = stdout.read_to_string(&mut s);
        let _ = tx.send(s);
    });
    let deadline = Instant::now() + timeout;
    loop {
        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(out) => {
                let _ = child.wait();
                return Some(out);
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                let _ = child.wait();
                return None;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
            }
        }
    }
}

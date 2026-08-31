//! Spawning an agent CLI with a hard timeout, and probing whether a CLI is
//! installed. A hung agent must never block the broker: [`run_cli`] kills the
//! child once the deadline passes and returns an error the broker can log.
use crew_hive::childproc::no_console_window;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// True if `program` resolves to an executable on `$PATH` (like `command -v`).
/// Cheap and exec-free so discovery of many agents stays fast.
pub fn on_path(program: &str) -> bool {
    if program.contains('/') {
        return is_exec(Path::new(program));
    }
    let Ok(path) = std::env::var("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| is_exec(&dir.join(program)))
}

fn is_exec(p: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        p.metadata()
            .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        p.is_file()
    }
}

/// Run `program args…` to completion, capturing stdout (stderr is discarded so
/// CLI banners never leak into a reply). Returns `Err` if the process can't be
/// spawned or doesn't finish within `timeout` (in which case it is killed).
pub fn run_cli(program: &str, args: &[String], timeout: Duration) -> Result<String, String> {
    let mut child = no_console_window(&mut Command::new(program))
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        // Captured, not discarded. An agent CLI that is installed but not
        // signed in says so on stderr and writes nothing to stdout, and
        // throwing that away left the single most likely failure reading
        // "produced no output" — true, and useless.
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to launch {program}: {e}"))?;

    // Drain stdout on a side thread so a large reply can't deadlock the pipe
    // while we poll for the deadline below.
    let stdout = child.stdout.take().expect("stdout was piped");
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut s = String::new();
        let mut r = stdout;
        let _ = r.read_to_string(&mut s);
        let _ = tx.send(s);
    });
    // Same for stderr, and for the same reason — an unread pipe that fills
    // would block the child forever. Only ever read when stdout came back
    // empty, so a chatty banner (codex prints one) never reaches a reply.
    let stderr = child.stderr.take().expect("stderr was piped");
    let (etx, erx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut s = String::new();
        let mut r = stderr;
        let _ = r.read_to_string(&mut s);
        let _ = etx.send(s);
    });

    let deadline = Instant::now() + timeout;
    loop {
        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(out) if !out.trim().is_empty() => {
                let _ = child.wait();
                return Ok(out);
            }
            // Empty stdout is a FAILURE, not an empty success. It used to
            // return `Ok("")` — so a CLI that had written its reason to
            // stderr and exited looked exactly like an agent with nothing to
            // say, and the reason was discarded unread. The relay caught the
            // emptiness one layer up and reported its own generic message.
            Ok(_) => {
                let _ = child.wait();
                let why = erx
                    .recv_timeout(Duration::from_millis(250))
                    .unwrap_or_default();
                return Err(explain_failure(program, &why));
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                let _ = child.wait();
                let why = erx
                    .recv_timeout(Duration::from_millis(250))
                    .unwrap_or_default();
                return Err(explain_failure(program, &why));
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("{program}: timed out after {timeout:?}"));
                }
            }
        }
    }
}

/// Why a CLI agent produced nothing, in words the user can act on.
///
/// The overwhelmingly likely cause is that the CLI is installed but not
/// signed in — crew adds `claude`, `codex` and `opencode` to the roster on
/// PATH presence alone (there is no cheap way to ask whether a session is
/// valid without spending a call), so "on the roster" and "usable" are not
/// the same thing. When stderr says as much, say what to do about it.
pub fn explain_failure(program: &str, stderr: &str) -> String {
    let tail: String = stderr
        .lines()
        .map(str::trim)
        .rfind(|l| !l.is_empty())
        .unwrap_or("")
        .chars()
        .take(160)
        .collect();
    let looks_like_auth = {
        let low = tail.to_lowercase();
        [
            "log in",
            "login",
            "sign in",
            "signin",
            "unauthorized",
            "not authenticated",
            "authentication",
            "api key",
            "credentials",
            "401",
        ]
        .iter()
        .any(|p| low.contains(p))
    };
    if looks_like_auth {
        return format!(
            "{program} is installed but not signed in \u{2014} run `{program}` once in a \
             terminal to log in ({tail})"
        );
    }
    if tail.is_empty() {
        return format!("{program}: produced no output");
    }
    format!("{program}: produced no output ({tail})")
}

#[cfg(test)]
#[path = "run_tests.rs"]
mod tests;

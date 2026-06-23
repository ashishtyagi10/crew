//! Spawning an agent CLI with a hard timeout, and probing whether a CLI is
//! installed. A hung agent must never block the broker: [`run_cli`] kills the
//! child once the deadline passes and returns an error the broker can log.
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
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
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

    let deadline = Instant::now() + timeout;
    loop {
        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(out) => {
                let _ = child.wait();
                return Ok(out);
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                let _ = child.wait();
                return Err(format!("{program}: produced no output"));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn on_path_finds_sh() {
        assert!(on_path("sh"));
    }

    #[test]
    fn on_path_rejects_missing() {
        assert!(!on_path("definitely-not-a-real-binary-xyz"));
    }

    #[test]
    fn run_cli_captures_stdout() {
        let args = vec!["-c".into(), "printf hello".into()];
        assert_eq!(
            run_cli("sh", &args, Duration::from_secs(5)).unwrap(),
            "hello"
        );
    }

    #[test]
    fn run_cli_times_out_on_hang() {
        let args = vec!["-c".into(), "sleep 5".into()];
        let r = run_cli("sh", &args, Duration::from_millis(150));
        assert!(r.unwrap_err().contains("timed out"));
    }
}

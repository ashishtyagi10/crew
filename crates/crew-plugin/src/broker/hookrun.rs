//! Running one hook: `sh -c` (`cmd /C` on Windows) in the project, its event on stdin, both
//! pipes drained on threads, the whole process group killed at the deadline. That is
//! `sysrun`'s discipline, which cannot be reused whole: it closes stdin and folds the exit
//! code into its text, and a hook is answered by both.
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crew_hive::childproc::no_console_window;

/// What one hook did.
#[derive(Debug)]
pub(super) enum Ran {
    /// It ran to an exit: the code, stdout, stderr.
    Exited(i32, String, String),
    /// It never ran, never finished, or a signal ended it: why, worded to follow its command.
    Failed(String),
}

/// Bytes kept per stream. A hook answers with a line of JSON or a linter's findings; past
/// this it is misbehaving, and its pipe is still drained so it cannot block on a full one.
const KEEP: usize = 64 * 1024;

/// How long the pipes get to close once the hook has exited.
const GRACE: Duration = Duration::from_millis(300);

/// The shell's word for "there was no command to run": `sh` says 126 (not runnable) or 127
/// (not found), `cmd` says 9009. The hook never ran, so its `on_failure` decides, not its exit.
const NOT_RUN: &[i32] = if cfg!(windows) { &[9009] } else { &[126, 127] };

pub(super) fn run(cmd: &str, cwd: &Path, input: &str, deadline: Duration) -> Ran {
    let mut c = shell(cmd);
    c.current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // Its own process group, so a deadline reaches whatever the hook started, not only `sh`.
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut c, 0);
    let mut child = match c.spawn() {
        Ok(child) => child,
        Err(e) => return Ran::Failed(format!("could not start ({e})")),
    };
    // On a thread: a hook that never reads its stdin must not stall the call on a full pipe.
    if let Some(mut w) = child.stdin.take() {
        let input = input.to_owned();
        std::thread::spawn(move || {
            let _ = w.write_all(input.as_bytes());
        });
    }
    let out = drain(child.stdout.take());
    let err = drain(child.stderr.take());
    let end = Instant::now() + deadline;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() >= end => {
                let _ = child.kill();
                let _ = child.wait();
                reap(child.id());
                return Ran::Failed(format!(
                    "ran past its {deadline:?} deadline and was stopped"
                ));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            Err(e) => return Ran::Failed(format!("could not be waited on ({e})")),
        }
    };
    // `sh` exiting does not empty its group: something it backgrounded can hold a pipe open.
    reap(child.id());
    let (out, err) = (
        out.recv_timeout(GRACE).unwrap_or_default(),
        err.recv_timeout(GRACE).unwrap_or_default(),
    );
    match status.code() {
        Some(code) if NOT_RUN.contains(&code) => Ran::Failed(format!(
            "could not run ({})",
            err.lines().next().unwrap_or("").trim()
        )),
        Some(code) => Ran::Exited(code, out, err),
        None => Ran::Failed("was ended by a signal".into()),
    }
}

#[cfg(not(windows))]
fn shell(cmd: &str) -> Command {
    let mut c = Command::new("/bin/sh");
    no_console_window(&mut c);
    c.args(["-c", cmd]);
    c
}

/// `raw_arg`, because `cmd` parses its own command line and Rust's quoting for an ordinary
/// argument would hand it a line it reads differently from the one the user wrote.
#[cfg(windows)]
fn shell(cmd: &str) -> Command {
    use std::os::windows::process::CommandExt;
    let mut c = Command::new("cmd");
    no_console_window(&mut c);
    c.arg("/C").raw_arg(cmd);
    c
}

#[cfg(unix)]
fn reap(pgid: u32) {
    crate::broker::sysrun::kill_group(pgid);
}

/// Windows has no process group to sweep here; `kill` already ended `cmd` on a deadline.
#[cfg(not(unix))]
fn reap(_: u32) {}

/// Read `pipe` to its end on a thread, keeping [`KEEP`] bytes, and send the text back. A
/// channel rather than a join, so a pipe something still holds open cannot hold the call.
fn drain(pipe: Option<impl Read + Send + 'static>) -> mpsc::Receiver<String> {
    let (tx, rx) = mpsc::channel();
    if let Some(mut pipe) = pipe {
        std::thread::spawn(move || {
            let (mut kept, mut buf) = (Vec::new(), [0u8; 8192]);
            while let Ok(n @ 1..) = pipe.read(&mut buf) {
                let room = KEEP.saturating_sub(kept.len());
                kept.extend_from_slice(&buf[..n.min(room)]);
            }
            let _ = tx.send(String::from_utf8_lossy(&kept).into_owned());
        });
    }
    rx
}

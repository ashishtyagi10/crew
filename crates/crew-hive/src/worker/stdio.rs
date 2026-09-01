//! [`StdioTransport`]: a sidecar process at the far end of the bridge.
//!
//! One child, one conversation at a time. The lock is not an implementation detail to optimise
//! away later — two tasks interleaving JSON lines on one pipe is a protocol violation with no
//! way back, and the alternative (a child per task) throws away the state that made a sidecar
//! worth having.
use std::io::BufReader;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::Mutex;

use crate::wire::{Host, RemoteReply, RemoteTask, Transport, TransportError};

/// A running sidecar.
pub struct StdioTransport {
    /// The child, kept so dropping this kills it rather than leaving an orphan holding a pipe.
    child: Mutex<Child>,
    io: Mutex<(BufReader<ChildStdout>, ChildStdin)>,
    /// What was spawned, for an error message that names it.
    command: String,
}

impl StdioTransport {
    /// Spawn `program args…` and speak the protocol to it.
    ///
    /// The child's stderr is INHERITED on purpose: a sidecar's traceback belongs on crew's
    /// stderr where a person will see it, and capturing it into a pipe nobody reads is how a
    /// crashed worker becomes a mysterious hang.
    pub fn spawn(program: &str, args: &[String]) -> std::io::Result<Self> {
        let mut cmd = Command::new(program);
        crate::childproc::no_console_window(&mut cmd);
        cmd.args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        let mut child = cmd.spawn()?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| std::io::Error::other("the worker has no stdout"))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| std::io::Error::other("the worker has no stdin"))?;
        let command = match args.is_empty() {
            true => program.to_string(),
            false => format!("{program} {}", args.join(" ")),
        };
        Ok(Self {
            child: Mutex::new(child),
            io: Mutex::new((BufReader::new(stdout), stdin)),
            command,
        })
    }

    /// Whether the sidecar is still alive. A worker that exited takes every later task with it,
    /// and saying so beats a write to a closed pipe.
    pub fn alive(&self) -> bool {
        let mut child = self.child.lock().unwrap_or_else(|e| e.into_inner());
        matches!(child.try_wait(), Ok(None))
    }

    /// Whether the sidecar has stopped, given a moment to.
    ///
    /// EOF on the child's stdout and the child's exit are two events, in that order, and asking
    /// `try_wait` the instant the pipe closes usually catches the process still there. Without
    /// the grace, "the worker exited" — the useful message — is reported as the generic one on
    /// most runs and the useful one on the rest.
    fn stopped(&self) -> bool {
        let mut child = self.child.lock().unwrap_or_else(|e| e.into_inner());
        for _ in 0..20 {
            match child.try_wait() {
                Ok(Some(_)) => return true,
                Ok(None) => std::thread::sleep(std::time::Duration::from_millis(10)),
                Err(_) => return false,
            }
        }
        false
    }

    /// What was spawned.
    pub fn command(&self) -> &str {
        &self.command
    }
}

impl Drop for StdioTransport {
    fn drop(&mut self) {
        let mut child = self.child.lock().unwrap_or_else(|e| e.into_inner());
        let _ = child.kill();
        let _ = child.wait();
    }
}

impl Transport for StdioTransport {
    fn dispatch<'a>(
        &'a self,
        task: RemoteTask,
        host: Host<'a>,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<RemoteReply, TransportError>> + Send + 'a>,
    > {
        Box::pin(async move {
            let mut io = self.io.lock().unwrap_or_else(|e| e.into_inner());
            let (reader, writer) = &mut *io;
            let out = super::converse(task, reader, writer, &host);
            match out {
                Err(TransportError::Recv(why)) if self.stopped() => Err(TransportError::Recv(
                    format!("the worker `{}` exited: {why}", self.command),
                )),
                other => other,
            }
        })
    }
}

/// Read a sidecar command out of a config string: `python3 -m crew_langgraph`. Empty or
/// whitespace means no sidecar, which is the default and the thing every probe checks first.
pub fn parse_command(raw: &str) -> Option<(String, Vec<String>)> {
    let mut parts = raw.split_whitespace().map(str::to_string);
    let program = parts.next().filter(|p| !p.is_empty())?;
    Some((program, parts.collect()))
}

/// Whether a sidecar command can be run at all: the program exists and is executable.
///
/// Probed rather than assumed, because "no Python on this machine" must be one line in
/// `/doctor` rather than a failed task at the end of a long graph.
pub fn probe(program: &str) -> bool {
    if std::path::Path::new(program).is_file() {
        return true;
    }
    // Bare name: look it up the way a shell would.
    let path = std::env::var_os("PATH").unwrap_or_default();
    std::env::split_paths(&path).any(|dir| {
        let p = dir.join(program);
        p.is_file() || p.with_extension("exe").is_file()
    })
}

#[cfg(test)]
#[path = "stdio_tests.rs"]
mod tests;

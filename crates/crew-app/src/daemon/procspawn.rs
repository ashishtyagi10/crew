//! The production [`Spawner`](super::session::Spawner): one broker child per session, plus the
//! [`SessionProc`] that wraps it. Split out of `session.rs` (the registry) so each file holds one
//! thing — the registry is testable without this file, and this file is the only one that spawns.
use std::path::{Path, PathBuf};

use super::session::{Buffer, SessionProc, Spawner};

/// The production spawner: one broker child per session, started the way the pane starts one
/// today — this binary re-exec'd with `--broker-plugin`, in its own process group so killing the
/// session takes the agent CLIs it spawned rather than orphaning them (the lesson `Plugin::drop`
/// already learned).
pub(crate) struct ProcSpawner {
    pub program: PathBuf,
    pub args: Vec<String>,
}

impl ProcSpawner {
    /// Spawn this crew binary as a broker, matching `chatspawn::crew_broker_cmd`'s resolution so
    /// a dev build's daemon runs a dev broker.
    pub(crate) fn broker() -> Self {
        Self {
            program: std::env::var_os("CREW_BROKER_PLUGIN")
                .map(PathBuf::from)
                .or_else(|| std::env::current_exe().ok())
                .unwrap_or_else(|| PathBuf::from("crew")),
            args: vec!["--broker-plugin".to_string()],
        }
    }
}

impl Spawner for ProcSpawner {
    fn spawn(
        &mut self,
        cwd: Option<&Path>,
        requester: Option<&str>,
    ) -> std::io::Result<Box<dyn SessionProc>> {
        // Windows: without the helper on the very next line, the daemon's broker child flashes
        // a console window — and crew-hive's source-tree guard
        // (`no_console_window_is_applied_at_every_spawn_site`) fails any spawn site that skips
        // it, which is how this one was caught.
        let mut cmd = std::process::Command::new(&self.program);
        crew_hive::childproc::no_console_window(&mut cmd);
        cmd.args(&self.args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null());
        if let Some(dir) = cwd.filter(|d| d.is_dir()) {
            cmd.current_dir(dir);
        }
        // The login-shell PATH, as the pane's own broker gets it: a daemon
        // broker on launchd's PATH would call every vendor CLI absent.
        for (k, v) in crate::spawn::hydrated_env() {
            cmd.env(k, v);
        }
        // Who the work is for. Without this the broker's gate sees no value, reads it as a local
        // pane, and trusts a remote sender exactly as much as a person at the keyboard.
        if let Some(r) = requester {
            cmd.env(crew_plugin::approval::REQUESTER_ENV, r);
        }
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            cmd.process_group(0);
        }
        Ok(Box::new(ChildProc::new(cmd.spawn()?)))
    }
}

/// A [`SessionProc`] backed by a real child process: stdin held for writes, stdout drained by a
/// reader thread into a shared buffer so the daemon never blocks on a chatty broker.
struct ChildProc {
    child: std::process::Child,
    stdin: Option<std::process::ChildStdin>,
    buf: std::sync::Arc<std::sync::Mutex<Buffer>>,
}

impl ChildProc {
    fn new(mut child: std::process::Child) -> Self {
        let buf = std::sync::Arc::new(std::sync::Mutex::new(Buffer::default()));
        if let Some(out) = child.stdout.take() {
            let sink = buf.clone();
            std::thread::spawn(move || {
                use std::io::BufRead;
                for line in std::io::BufReader::new(out).lines().map_while(Result::ok) {
                    let mut b = sink.lock().unwrap_or_else(|e| e.into_inner());
                    b.push(line);
                }
            });
        }
        let stdin = child.stdin.take();
        Self { child, stdin, buf }
    }
}

impl SessionProc for ChildProc {
    fn alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }

    fn send(&mut self, line: &str) -> bool {
        use std::io::Write;
        let Some(w) = self.stdin.as_mut() else {
            return false;
        };
        let ok = w
            .write_all(line.as_bytes())
            .and_then(|_| w.write_all(b"\n"))
            .and_then(|_| w.flush())
            .is_ok();
        if !ok {
            self.stdin = None; // a broken pipe never heals; stop pretending it might
        }
        ok
    }

    fn output(&self) -> (Vec<String>, usize) {
        let b = self.buf.lock().unwrap_or_else(|e| e.into_inner());
        (b.lines.clone(), b.dropped)
    }
}

//! The daemon's session registry: the resident owns the agent processes, not the pane.
//!
//! Today a `/crew` pane spawns its own broker child (`chatspawn::crew_broker_cmd`) and kills it on
//! drop, which is exactly why closing the window ends the work. Here the daemon holds them, so a
//! session outlives whatever asked for it.
//!
//! The process itself sits behind [`Spawner`]/[`SessionProc`] so tests exercise the registry —
//! open, list, close, reap — without spawning real brokers, and so 1.3 can swap in a spawner that
//! also bridges stdio.
use std::path::{Path, PathBuf};

/// A live session's process, as the registry needs to see it.
pub(crate) trait SessionProc: Send {
    /// Still running? Reaps internally, so a child that exited on its own is reported dead.
    fn alive(&mut self) -> bool;
    /// Terminate and reap. Idempotent.
    fn kill(&mut self);
}

/// How a session's process is started. The seam that keeps tests off real subprocesses.
pub(crate) trait Spawner: Send {
    fn spawn(&mut self, cwd: Option<&Path>) -> std::io::Result<Box<dyn SessionProc>>;
}

/// One registered session.
pub(crate) struct Session {
    pub id: String,
    pub label: String,
    pub cwd: Option<PathBuf>,
    proc: Box<dyn SessionProc>,
}

/// What a `sessions` reply says about one session.
#[derive(Debug, PartialEq, Clone)]
pub(crate) struct Card {
    pub id: String,
    pub label: String,
    pub cwd: Option<String>,
    pub alive: bool,
}

/// The set of sessions this daemon owns. Ids are `s1`, `s2`, … from a monotonic counter: unique
/// within a daemon's life, stable across a test run, and never reused after a close (a reused id
/// would let a stale client's close land on somebody else's session).
pub(crate) struct Registry {
    sessions: Vec<Session>,
    next: u64,
    spawner: Box<dyn Spawner>,
}

impl Registry {
    pub(crate) fn new(spawner: Box<dyn Spawner>) -> Self {
        Self {
            sessions: Vec::new(),
            next: 1,
            spawner,
        }
    }

    /// Start a session. The label is cosmetic; the id is the handle.
    pub(crate) fn open(&mut self, label: &str, cwd: Option<&Path>) -> std::io::Result<String> {
        let proc = self.spawner.spawn(cwd)?;
        let id = format!("s{}", self.next);
        self.next += 1;
        self.sessions.push(Session {
            id: id.clone(),
            label: label.to_string(),
            cwd: cwd.map(Path::to_path_buf),
            proc,
        });
        Ok(id)
    }

    /// Every session, dead ones included — a session whose process died is still a fact the
    /// caller needs, and silently hiding it would read as "it was never opened".
    pub(crate) fn cards(&mut self) -> Vec<Card> {
        self.sessions
            .iter_mut()
            .map(|s| Card {
                id: s.id.clone(),
                label: s.label.clone(),
                cwd: s.cwd.as_ref().map(|p| p.display().to_string()),
                alive: s.proc.alive(),
            })
            .collect()
    }

    /// Close and forget one session. Returns whether it was still running — distinguishing "I
    /// stopped it" from "it had already died" — or `None` if no such id.
    pub(crate) fn close(&mut self, id: &str) -> Option<bool> {
        let idx = self.sessions.iter().position(|s| s.id == id)?;
        let mut s = self.sessions.remove(idx);
        let was_alive = s.proc.alive();
        s.proc.kill();
        Some(was_alive)
    }

    /// How many sessions are registered (dead included).
    pub(crate) fn len(&mut self) -> usize {
        self.sessions.len()
    }
}

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
    fn spawn(&mut self, cwd: Option<&Path>) -> std::io::Result<Box<dyn SessionProc>> {
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
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            cmd.process_group(0);
        }
        Ok(Box::new(ChildProc {
            child: cmd.spawn()?,
        }))
    }
}

/// A [`SessionProc`] backed by a real child process.
struct ChildProc {
    child: std::process::Child,
}

impl SessionProc for ChildProc {
    fn alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;

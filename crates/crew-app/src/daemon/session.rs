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
    /// Write one line to the process's stdin. `false` if the pipe is gone.
    fn send(&mut self, line: &str) -> bool;
    /// Everything the process has printed since the session opened, oldest first, together with
    /// how many lines were dropped off the front to stay inside [`BUFFER_LINES`].
    fn output(&self) -> (Vec<String>, usize);
}

/// How many output lines a session keeps. A broker under load prints steadily and the daemon
/// outlives every client, so this buffer is unbounded in TIME — it must be bounded in size or a
/// week-old resident is a memory leak. Old lines fall off the front and are COUNTED, so a client
/// that reattaches after a long absence is told it missed some rather than being handed a
/// gap-with-no-marker and quietly drawing a false history.
pub(crate) const BUFFER_LINES: usize = 2000;

/// The shared output buffer a session's reader thread fills.
#[derive(Default)]
pub(crate) struct Buffer {
    lines: Vec<String>,
    dropped: usize,
}

impl Buffer {
    pub(crate) fn push(&mut self, line: String) {
        self.lines.push(line);
        if self.lines.len() > BUFFER_LINES {
            let over = self.lines.len() - BUFFER_LINES;
            self.lines.drain(..over);
            self.dropped += over;
        }
    }

    /// Lines from absolute index `after` onward, plus the next cursor. A cursor pointing into
    /// dropped territory is clamped forward to the oldest line still held — the client is told
    /// what it missed through `dropped`, and never silently rewound to replay old output.
    pub(crate) fn since(&self, after: usize) -> (Vec<String>, usize, usize) {
        let start = after.max(self.dropped).min(self.dropped + self.lines.len());
        let from = start - self.dropped;
        (
            self.lines[from..].to_vec(),
            self.dropped + self.lines.len(),
            self.dropped,
        )
    }
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

    /// Send one line to a session's process. `None` if no such id, `Some(false)` if the pipe
    /// is gone (the process died).
    pub(crate) fn send(&mut self, id: &str, line: &str) -> Option<bool> {
        let s = self.sessions.iter_mut().find(|s| s.id == id)?;
        Some(s.proc.send(line))
    }

    /// A session's output from cursor `after`: the lines, the next cursor, and how many were
    /// dropped from the front of the buffer over the session's life.
    pub(crate) fn output(&self, id: &str, after: usize) -> Option<(Vec<String>, usize, usize)> {
        let s = self.sessions.iter().find(|s| s.id == id)?;
        let (lines, dropped) = s.proc.output();
        Some(Buffer { lines, dropped }.since(after))
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

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;

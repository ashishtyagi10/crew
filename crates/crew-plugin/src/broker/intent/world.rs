//! The world the classifier routes in. Until this file the routing prompt
//! held ONLY the message: the model picked `fan` without knowing how many
//! agents there were, `commit` without knowing whether anything had
//! changed, `swarm` without knowing which tools existed. A classifier that
//! cannot see the room guesses at the room. Three cheap facts fix that —
//! the roster's names, whether the tree is dirty, the tool surface — each
//! said only when known, so an empty world leaves the prompt byte-identical
//! to the bare grammar.
//!
//! Gathering is bounded on purpose: no model call, no network, and the one
//! subprocess (`git status`) is killed at [`GIT_BUDGET`] — the router is
//! overhead before the real work, and a slow fact is worth less than none.
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::broker::route::clip;
use crate::broker::session::Session;

/// How long the dirty-tree probe may take before it is dropped.
const GIT_BUDGET: Duration = Duration::from_secs(1);

/// The tool line is one line: the capabilities are a sentence per source,
/// and the classifier needs the names, not the catalogue.
const TOOLS_MAX: usize = 400;

/// What the router knows about its surroundings; every field optional.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct World {
    /// Roster names, in roster order — also the set an `AGENTS:` line may
    /// pick from, so the model can only name what it was shown.
    pub(crate) agents: Vec<String>,
    /// Paths that differ from HEAD (crew's own `.crew/` excluded), or `None`
    /// when there is no repository or the probe ran out of time.
    pub(crate) dirty: Option<usize>,
    /// One capability line per tool source (`Tools::capabilities`).
    pub(crate) tools: Vec<String>,
    /// What the user asked last turn (`thread::RECENT_CAP` chars), so the
    /// router can tell a follow-up ("shorter", "now the tests too") from a
    /// fresh request; `None` on the first turn.
    pub(crate) recent: Option<String>,
}

impl World {
    /// The facts this session can give cheaply: the registry's names, one
    /// bounded `git status` in the project dir, the tool surface the planner
    /// already sees.
    pub(crate) fn gather(session: &Session) -> World {
        World {
            agents: session.registry().names(),
            dirty: crate::broker::gitmsg::project_dir()
                .ok()
                .and_then(|d| dirty_count(&d)),
            tools: session
                .tools()
                .map(|t| t.capabilities())
                .unwrap_or_default(),
            recent: crate::broker::thread::lock(&session.thread).recent(),
        }
    }

    /// The prompt block — one labelled line per known fact, ending in a blank
    /// line so the message follows it; the empty string when nothing is
    /// known, so the prompt without a world is the prompt as it always was.
    pub(crate) fn section(&self) -> String {
        let mut lines: Vec<String> = Vec::new();
        if !self.agents.is_empty() {
            lines.push(format!("agents: {}", self.agents.join(", ")));
        }
        match self.dirty {
            Some(0) => lines.push("tree: clean".to_string()),
            Some(1) => lines.push("tree: dirty (1 file)".to_string()),
            Some(n) => lines.push(format!("tree: dirty ({n} files)")),
            None => {}
        }
        if !self.tools.is_empty() {
            lines.push(format!(
                "tools: {}",
                clip(&self.tools.join("; "), TOOLS_MAX)
            ));
        }
        if let Some(asked) = &self.recent {
            lines.push(format!("last turn: {asked}"));
        }
        if lines.is_empty() {
            return String::new();
        }
        format!("The world you route in:\n{}\n\n", lines.join("\n"))
    }
}

/// Changed paths under `dir`, or `None` outside a repository, on any git
/// error, or past [`GIT_BUDGET`]. The child is polled rather than waited on
/// so the budget is real: a `git status` on a cold network mount can hang,
/// and this call sits on the path of every plain message.
pub(crate) fn dirty_count(dir: &Path) -> Option<usize> {
    let mut cmd = Command::new("git");
    crew_hive::childproc::no_console_window(&mut cmd);
    let mut child = cmd
        .args([
            "status",
            "--porcelain",
            "--",
            crate::broker::changed::NOT_CREW,
        ])
        .current_dir(dir)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut out = child.stdout.take()?;
    // A reader thread keeps the pipe drained, so a large status can never
    // block the child and turn a bounded probe into a hang.
    let reader = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = out.read_to_string(&mut s);
        s
    });
    let deadline = Instant::now() + GIT_BUDGET;
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => break,
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(10));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
    let text = reader.join().ok()?;
    Some(text.lines().filter(|l| !l.trim().is_empty()).count())
}

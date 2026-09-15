//! The check crew runs on its own work.
//!
//! Every other agentic tool worth copying ends a coding task by running
//! something: Codex runs the test command, Cline runs the build, and both do
//! it because a model's own account of what it did is the least reliable
//! signal available. Crew reported the diff and the language server's
//! diagnostics and stopped there — accurate about SYNTAX and silent about
//! whether the thing still builds.
//!
//! So: a project that declares a check command in `.crew/check` gets it run
//! after every task that changed files, and the result said in one line. It
//! is declared rather than guessed for one reason — the command spends your
//! CPU and can take minutes, and a tool that starts doing that unasked is a
//! tool people turn off. When crew can SEE what the command would probably
//! be (a `Cargo.toml`, a `package.json`) it says so once and leaves the
//! decision alone.
use std::path::{Path, PathBuf};

use crate::PluginEvent;

use super::relay::msg;
use super::session::Session;

/// Lines of a failing command's output shown. Enough to name the first
/// error; the whole thing is one `sys:run` away for whoever wants it.
const FAIL_LINES: usize = 6;

/// What a project would likely run, by the file that gives it away. Only
/// used for the one-time note — never run on its own.
const DETECTED: &[(&str, &str)] = &[
    ("Cargo.toml", "cargo check --workspace"),
    ("package.json", "npm test"),
    ("pyproject.toml", "pytest -q"),
    ("Makefile", "make"),
];

fn base_dir() -> PathBuf {
    std::env::var("CREW_PROJECT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

/// The declared command: the first non-empty, non-comment line of
/// `.crew/check`.
pub(crate) fn command_at(base: &Path) -> Option<String> {
    let text = std::fs::read_to_string(base.join(".crew").join("check")).ok()?;
    text.lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_string)
}

/// What this project probably checks with, for the one-time note.
pub(crate) fn detected_at(base: &Path) -> Option<&'static str> {
    DETECTED
        .iter()
        .find(|(marker, _)| base.join(marker).exists())
        .map(|(_, cmd)| *cmd)
}

/// How a check ended. `sys:run` answers `Ok("exit 3\n…")` for a command that
/// RAN and failed, and `Err(…)` only for one that could not run at all — so
/// the verdict is the exit line, not the Result.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Outcome {
    pub ok: bool,
    pub text: String,
}

pub(crate) fn outcome(result: Result<String, String>) -> Outcome {
    match result {
        Ok(text) => Outcome {
            ok: text.starts_with("exit 0\n") || text.trim() == "exit 0",
            text,
        },
        Err(e) => Outcome { ok: false, text: e },
    }
}

/// The line for a finished run: passed in one line, failed with the head of
/// the output — the part that names the first error.
pub(crate) fn line(cmd: &str, o: &Outcome) -> String {
    if o.ok {
        return format!("check: {cmd} \u{2014} passed");
    }
    let head: Vec<&str> = o
        .text
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with("exit "))
        .take(FAIL_LINES)
        .collect();
    format!("check: {cmd} \u{2014} FAILED\n{}", head.join("\n"))
}

/// The one-time note, when there is a command to suggest and none declared.
pub(crate) fn note(cmd: &str) -> String {
    format!(
        "tip: crew can run a check after every task that changes files \
         \u{2014} put a command in .crew/check (this looks like `{cmd}`)"
    )
}

/// Run the declared check after a task that changed files, and say how it
/// went. Nothing declared, nothing changed, or no shell (Windows): silent.
pub(crate) fn after_task(
    session: &Session,
    changed: bool,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    if !changed {
        return Ok(());
    }
    let base = base_dir();
    let Some(cmd) = command_at(&base) else {
        // Say once that this is possible, and only when we can name the
        // command — an unsolicited tip that cannot be acted on is noise.
        let told = session
            .announced_check
            .swap(true, std::sync::atomic::Ordering::Relaxed);
        return match (told, detected_at(&base)) {
            (false, Some(c)) => emit(msg("agent smith", note(c))),
            _ => Ok(()),
        };
    };
    let o = outcome(run(&cmd));
    // What the check said, into the graph: "the tests fail on this" is a
    // fact about this project, and the next session should not have to
    // rediscover it.
    super::recall::record(
        &session.recall,
        &format!("check: {cmd}"),
        Some(match o.ok {
            true => "passed".to_string(),
            false => format!(
                "failed: {}",
                o.text.lines().take(2).collect::<Vec<_>>().join(" ")
            ),
        })
        .as_deref(),
    );
    emit(msg("agent smith", line(&cmd, &o)))
}

#[cfg(unix)]
fn run(cmd: &str) -> Result<String, String> {
    super::sysrun::run(cmd)
}

/// No `/bin/sh`, no check — `sys:run` says the same on Windows.
#[cfg(not(unix))]
fn run(cmd: &str) -> Result<String, String> {
    super::sysrun::run(cmd)
}

#[cfg(test)]
#[path = "selfcheck_tests.rs"]
mod tests;

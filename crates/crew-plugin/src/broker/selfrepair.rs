//! One pass at its own breakage.
//!
//! `selfcheck` says whether the project's check passed. This is what crew
//! does when it did not: hand the command, the output and the instruction
//! back to the crew that caused it, once, and say how that went.
//!
//! Bounded on purpose. ONE pass — a repair changes files, which would run the
//! check, which could fail, which would start another pass, and a loop that
//! spends money while you are asleep is not autonomy. Never re-entered
//! (`Session::repairing`). Never a commit. And always undoable: a checkpoint
//! was taken before the task that broke the build, so "undo that" puts the
//! whole thing back — the original change and the repair together.
use std::sync::atomic::Ordering;

use crate::PluginEvent;

use super::relay::msg;
use super::selfcheck::{line, outcome, run, Outcome};
use super::session::Session;

/// Lines of the failure handed to the repair pass — more than the pane
/// shows, since the agent has to work from it.
const REPAIR_LINES: usize = 40;

/// A repair pass: the task crew gives itself when its own check failed.
/// Passed in as a closure so this file never has to know which engine
/// answers it, and so a test can watch what was asked without running one.
pub(crate) type Repair<'a> = &'a mut dyn FnMut(&str) -> anyhow::Result<()>;

/// The failing check, handed back to the crew that caused it — once.
///
/// This is the autonomy the check exists for: an agent that breaks the build
/// and stops is an agent you have to babysit, and everything needed to fix it
/// — the command, the output, the diff — is already here. Bounded hard: ONE
/// pass, never re-entered (the pass changes files, which would run the check,
/// which would fail, which would start another pass), and the tree it edits
/// is one "undo that" away, because a checkpoint was taken before the task
/// that broke it.
///
/// Off with a sentence — "don't fix it yourself" — because whether a machine
/// may act unasked is the user's call, not a config file's.
pub(crate) fn take_one_pass(
    session: &Session,
    cmd: &str,
    o: &Outcome,
    repair: Repair<'_>,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    if !autofix(session) || session.repairing.swap(true, Ordering::Relaxed) {
        return Ok(());
    }
    emit(msg(
        "agent smith",
        "taking one pass at it \u{2014} say \u{201c}don't fix it yourself\u{201d} to stop this",
    ))?;
    let asked = repair(&repair_task(cmd, o));
    let after = outcome(run(cmd));
    session.repairing.store(false, Ordering::Relaxed);
    asked?;
    emit(msg(
        "agent smith",
        match after.ok {
            true => format!("check: {cmd} \u{2014} passed after one pass"),
            false => format!("{}\n(one pass was not enough)", line(cmd, &after)),
        },
    ))
}

/// The prompt the repair pass gets: the command, what it said, and the one
/// instruction that keeps the pass honest.
pub(crate) fn repair_task(cmd: &str, o: &Outcome) -> String {
    let head: Vec<&str> = o.text.lines().take(REPAIR_LINES).collect();
    format!(
        "The project's own check failed after the change you just made.\n\n\
         Command: {cmd}\nOutput:\n{}\n\n\
         Fix the cause. Change as little as possible, do not weaken or delete \
         the check itself, and do not commit anything.",
        head.join("\n")
    )
}

/// Whether crew may take that pass. On unless the session said not to.
pub(crate) fn autofix(session: &Session) -> bool {
    !session.no_autofix.load(Ordering::Relaxed)
}

/// The words that turn the pass off and on, matched exactly like every other
/// human gate — "don't fix it yourself" must never be read as a task.
pub(crate) fn asks(task: &str) -> Option<bool> {
    let t = task
        .trim()
        .trim_end_matches(['.', '!'])
        .to_ascii_lowercase()
        .replace('\u{2019}', "'");
    const OFF: &[&str] = &[
        "don't fix it yourself",
        "dont fix it yourself",
        "stop fixing it yourself",
        "don't auto fix",
        "no auto fix",
    ];
    const ON: &[&str] = &["fix it yourself", "auto fix", "fix your own breakage"];
    if OFF.contains(&t.as_str()) {
        return Some(false);
    }
    ON.contains(&t.as_str()).then_some(true)
}

/// The router's one call.
pub(crate) fn gate(
    task: &str,
    session: &Session,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> Option<anyhow::Result<()>> {
    let on = asks(task)?;
    session.no_autofix.store(!on, Ordering::Relaxed);
    Some(emit(msg(
        "agent smith",
        match on {
            true => "I'll take one pass at a failing check myself",
            false => "I'll report a failing check and leave it to you",
        },
    )))
}

#[cfg(test)]
#[path = "selfrepair_tests.rs"]
mod tests;

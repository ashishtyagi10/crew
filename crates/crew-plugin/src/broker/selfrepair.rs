//! One pass at its own breakage.
//!
//! `selfcheck` says whether the project's check passed. This is what crew
//! does when it did not: hand the command, the part of the output that names
//! the failure, the change that caused it and the instruction back to the
//! crew that caused it, once, and say how that went.
//!
//! Bounded on purpose. ONE pass — a repair changes files, which would run the
//! check, which could fail, which would start another pass, and a loop that
//! spends money while you are asleep is not autonomy. Never re-entered
//! (`Session::repairing`). Never a commit. And always undoable: a checkpoint
//! was taken before the task that broke the build, so "undo that" puts the
//! whole thing back — the original change and the repair together.
use std::path::Path;
use std::sync::atomic::Ordering;

use crate::PluginEvent;

use super::relay::msg;
use super::selfcheck::{line, outcome, run, Outcome};
use super::session::Session;

/// Chars of the task's diff the repair pass is handed. Half the pane's
/// [`super::taskdiff::PATCH_CAP`]: the pass needs to see what it changed,
/// not reread it, and the failure is the other half of the prompt.
const CHANGE_CAP: usize = 6_000;

/// A failed check, as the repair pass is briefed on it.
pub(crate) struct Failure<'a> {
    pub cmd: &'a str,
    pub o: &'a Outcome,
    /// The repeat sentence, when the graph has seen this failure before.
    pub seen: Option<&'a str>,
    /// Whether the check's verdict before this one was a pass.
    pub passed_before: bool,
    /// Where the task ran: the tree its diff is taken in.
    pub dir: Option<&'a Path>,
}

/// A repair pass: the task crew gives itself when its own check failed.
/// Passed in as a closure so this file never has to know which engine
/// answers it, and so a test can watch what was asked without running one.
pub(crate) type Repair<'a> = &'a mut dyn FnMut(&str) -> anyhow::Result<()>;

/// The failing check, handed back to the crew that caused it — once.
///
/// This is the autonomy the check exists for: an agent that breaks the build
/// and stops is an agent you have to babysit, and everything needed to fix it
/// — the command, the output, the diff — is already here, and all three go
/// into the brief. Bounded hard: ONE pass, never re-entered (the pass changes
/// files, which would run the check, which would fail, which would start
/// another pass), and the tree it edits is one "undo that" away, because a
/// checkpoint was taken before the task that broke it.
///
/// Off with a sentence — "don't fix it yourself" — because whether a machine
/// may act unasked is the user's call, not a config file's.
pub(crate) fn take_one_pass(
    session: &Session,
    f: &Failure<'_>,
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
    // Taken after the gate: a pass the user turned off costs no git at all.
    let change = f.dir.and_then(|d| change_of(d, session));
    let asked = repair(&repair_task(f, change.as_deref()));
    let cmd = f.cmd;
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

/// The prompt the repair pass gets: the command, the part of its output that
/// names the failure, the change that caused it, and the one instruction
/// that keeps the pass honest.
pub(crate) fn repair_task(f: &Failure<'_>, change: Option<&str>) -> String {
    // The repeat goes in the prompt, not just the pane: a pass that knows the
    // project has broken this way before — and where it was last time — is
    // the whole reason the graph holds the verdicts at all.
    let memory = f
        .seen
        .map_or(String::new(), |s| format!("\n\nWhat crew remembers: {s}."));
    // The change is the first thing a repair needs: a check that passed
    // before it and fails after it failed BECAUSE of it. Said that way only
    // when the graph's last verdict was a pass; otherwise it is just the
    // change, and the pass has to judge.
    let change = change.map_or(String::new(), |p| {
        let lead = match f.passed_before {
            true => "The check passed before this change:",
            false => "This is the change the task made:",
        };
        format!("\n\n{lead}\n{}", super::taskdiff::fenced(p))
    });
    format!(
        "The project's own check failed after the change you just made.\n\n\
         Command: {}\nOutput (the failure, then the end):\n{}{change}{memory}\n\n\
         Fix the cause. Change as little as possible, do not weaken or delete \
         the check itself, and do not commit anything.",
        f.cmd,
        super::failexcerpt::excerpt(&f.o.text),
    )
}

/// What the task changed, from the tree pinned before it ran
/// (`Session::last_tree`, the same base the pane's diff is taken against),
/// clipped. `None` outside git, before any checkpoint, or when the tree is
/// back where it started.
fn change_of(dir: &Path, session: &Session) -> Option<String> {
    let base = session
        .last_tree
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()?;
    let patch = super::changed::patch(dir, &base).ok()?;
    (!patch.trim().is_empty()).then(|| super::taskdiff::clip(&patch, CHANGE_CAP, ""))
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

#[cfg(test)]
#[path = "selfrepair_brief_tests.rs"]
mod brief_tests;

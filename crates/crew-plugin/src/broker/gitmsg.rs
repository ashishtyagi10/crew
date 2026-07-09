//! `/commit` — AI-written commit messages (à la Aider / Cursor): the coder
//! agent reads the working tree's diff (staged wins; unstaged otherwise) and
//! drafts a Conventional Commits message. Nothing is committed until the user
//! runs `/commit apply` — the propose/apply split mirrors plan mode, and the
//! pending proposal is shared session state for the same reason.
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex, MutexGuard};

use crate::PluginEvent;

use super::relay::msg;
use super::session::{call_timeout, Session};
use super::stdio::roster;

/// Diff budget interpolated into the prompt (chars); clipped with a marker so
/// a huge refactor still yields a message instead of blowing the context.
const DIFF_CAP: usize = 12_000;

/// A drafted commit message awaiting `/commit apply`.
pub(crate) struct PendingCommit {
    pub message: String,
    /// True when the proposal covered the staged diff (`git commit -m`);
    /// false for unstaged tracked changes (`git commit -am`).
    pub staged: bool,
}

/// The session's pending commit proposal, shared like the pending plan.
pub(crate) type SharedCommit = Arc<Mutex<Option<PendingCommit>>>;

fn lock(c: &SharedCommit) -> MutexGuard<'_, Option<PendingCommit>> {
    c.lock().unwrap_or_else(|e| e.into_inner())
}

/// Run `git args` in `dir`, capturing stdout; non-zero exit becomes Err with
/// stderr folded in.
fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .map_err(|e| format!("git: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// The diff a commit message should describe: the staged diff when anything
/// is staged, else the unstaged tracked diff. `Ok(None)` for a clean tree;
/// `Err` outside a repository. The diff is clipped to [`DIFF_CAP`].
pub(crate) fn pick_diff(dir: &Path) -> Result<Option<(String, bool)>, String> {
    let clipped = |mut d: String| {
        if d.len() > DIFF_CAP {
            let mut cut = DIFF_CAP;
            while !d.is_char_boundary(cut) {
                cut -= 1;
            }
            d.truncate(cut);
            d.push_str("\n… (diff clipped)");
        }
        d
    };
    let staged = git(dir, &["diff", "--cached"])?;
    if !staged.trim().is_empty() {
        return Ok(Some((clipped(staged), true)));
    }
    let unstaged = git(dir, &["diff"])?;
    if !unstaged.trim().is_empty() {
        return Ok(Some((clipped(unstaged), false)));
    }
    Ok(None)
}

/// The one-completion prompt for the coder.
pub(crate) fn commit_prompt(diff: &str) -> String {
    format!(
        "Write a Conventional Commits message for this diff: an imperative \
         `type(scope): subject` line (≤72 chars), then — only if the change \
         needs it — a blank line and a short body explaining WHY.\n\
         Reply with the commit message only: no prose around it, no code \
         fences.\n\nDIFF:\n{diff}"
    )
}

/// Distill a model reply to the bare commit message: surrounding code fences
/// and a leading "commit message:"-style label are stripped; inner newlines
/// (subject + body) survive.
pub(crate) fn clean_message(reply: &str) -> String {
    let mut lines: Vec<&str> = reply.trim().lines().collect();
    // surrounding fence pair
    if lines.first().is_some_and(|l| l.trim().starts_with("```"))
        && lines.last().is_some_and(|l| l.trim().starts_with("```"))
        && lines.len() >= 2
    {
        lines = lines[1..lines.len() - 1].to_vec();
    }
    // a leading "commit message:"-style label line
    if lines
        .first()
        .is_some_and(|l| l.trim().to_lowercase().trim_end_matches(':') == "commit message")
    {
        lines.remove(0);
    }
    lines.join("\n").trim().to_string()
}

/// Create the commit: `git commit -m` for a staged proposal, `git commit -am`
/// for an unstaged one. Returns git's one-line summary.
pub(crate) fn do_commit(dir: &Path, message: &str, staged: bool) -> Result<String, String> {
    let args: &[&str] = if staged {
        &["commit", "-m", message]
    } else {
        &["commit", "-am", message]
    };
    let out = git(dir, args)?;
    Ok(out.lines().next().unwrap_or("committed").to_string())
}

/// `/commit` — propose a message for the current diff; `/commit apply` —
/// create the commit from the stored proposal.
pub(crate) fn commit_cmd(
    session: &mut Session,
    rest: &str,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let dir = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => return emit(msg("crew", format!("commit: no working directory: {e}"))),
    };
    if rest.trim() == "apply" {
        let pending = lock(&session.commit).take();
        let Some(p) = pending else {
            return emit(msg("crew", "no proposal — run /commit first"));
        };
        let m = match do_commit(&dir, &p.message, p.staged) {
            Ok(s) => s,
            Err(e) => format!("commit failed: {e}"),
        };
        return emit(msg("crew", m));
    }
    if !rest.trim().is_empty() {
        return emit(msg(
            "crew",
            "usage: /commit — propose · /commit apply — run it",
        ));
    }
    let (diff, staged) = match pick_diff(&dir) {
        Err(e) => return emit(msg("crew", format!("commit: {e}"))),
        Ok(None) => {
            return emit(msg(
                "crew",
                "nothing to commit — the tree is clean (stage or edit something)",
            ))
        }
        Ok(Some(d)) => d,
    };
    let reg = session.registry();
    if reg.is_empty() {
        return emit(msg("crew", roster(&reg)));
    }
    let author = if reg.get("coder").is_some() {
        "coder".to_string()
    } else {
        reg.names().first().cloned().unwrap_or_default()
    };
    emit(msg(
        "crew",
        format!(
            "drafting a commit message for the {} diff…",
            if staged { "staged" } else { "unstaged" }
        ),
    ))?;
    emit(PluginEvent::Activity {
        agent: author.clone(),
        state: "thinking".into(),
        from: "commit".into(),
    })?;
    let reply = reg
        .get(&author)
        .map(|a| a.call(&commit_prompt(&diff), call_timeout()));
    emit(PluginEvent::Activity {
        agent: String::new(),
        state: "idle".into(),
        from: String::new(),
    })?;
    let message = match reply {
        Some(Ok(r)) => clean_message(&r),
        Some(Err(e)) => return emit(msg("crew", format!("commit draft failed: {e}"))),
        None => return emit(msg("crew", "commit stopped — the coder went missing")),
    };
    if message.is_empty() {
        return emit(msg("crew", "the draft came back empty — try again"));
    }
    emit(msg(&format!("{author} → user"), message.clone()))?;
    *lock(&session.commit) = Some(PendingCommit { message, staged });
    emit(msg(
        "crew",
        "proposal ready — /commit apply creates the commit, /commit re-drafts",
    ))
}

#[cfg(test)]
#[path = "gitmsg_tests.rs"]
mod tests;

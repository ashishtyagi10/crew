//! `/commit` — AI-written commit messages (à la Aider / Cursor): the coder
//! agent reads the working tree's diff (staged wins; the whole tree otherwise,
//! new files included) and
//! drafts a Conventional Commits message. Nothing is committed until the user
//! runs `/commit apply` — the propose/apply split mirrors plan mode, and the
//! pending proposal is shared session state for the same reason.
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex, MutexGuard};

use crate::PluginEvent;

use super::constructs::{is_writer, pick_by_role};
use super::relay::msg;
use super::session::{call_timeout, Session};
use super::stdio::roster;

/// Diff budget interpolated into the prompt (chars); clipped with a marker so
/// a huge refactor still yields a message instead of blowing the context.
const DIFF_CAP: usize = 12_000;

/// The repo the diff-reading capabilities (commit, review, standup) work in.
/// Mirrors `sessionlog::base_dir` exactly, and for the same reason:
/// `CREW_PROJECT_DIR` is the test seam — lib tests share one process CWD (the
/// crate's own checkout!), so without it a commit test would stage and commit
/// the developer's real working tree. Production never sets it: the broker's
/// CWD *is* the project.
pub(crate) fn project_dir() -> Result<std::path::PathBuf, String> {
    if let Ok(d) = std::env::var("CREW_PROJECT_DIR") {
        return Ok(std::path::PathBuf::from(d));
    }
    std::env::current_dir().map_err(|e| format!("no working directory: {e}"))
}

/// A drafted commit message awaiting the user's conversational "apply".
pub(crate) struct PendingCommit {
    pub message: String,
    /// True when the proposal covered only what is staged; false when it
    /// covered the whole working tree, which `apply` must then stage.
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

/// Everything in the working tree that differs from the last commit,
/// untracked files included, as a patch — the comparison `/diff` shows and the
/// end-of-task note summarises, so all three describe the same work.
///
/// `git diff` cannot see a file git has never been told about, which is what
/// an agent produces every time it writes a new module. Building the
/// comparison through a throwaway index is the only way to include one without
/// touching the user's real index.
fn whole_tree_diff(dir: &Path) -> Result<String, String> {
    let tree = super::checkpoint::worktree_tree(dir)?;
    let base = super::checkpoint::git(dir, &["rev-parse", "--verify", "HEAD^{tree}"], None)
        .or_else(|_| {
            super::checkpoint::git(dir, &["hash-object", "-t", "tree", "/dev/null"], None)
        })?;
    super::checkpoint::git(
        dir,
        &[
            "diff-tree",
            "-p",
            "-r",
            &base,
            &tree,
            "--",
            super::changed::NOT_CREW,
        ],
        None,
    )
}

/// The diff a commit message should describe: the staged diff when anything
/// is staged, else everything that differs from the last commit. `Ok(None)`
/// for a clean tree; `Err` outside a repository. Clipped to [`DIFF_CAP`].
///
/// STAGED STILL WINS AND STILL MEANS ONLY WHAT IS STAGED. Someone who has
/// staged a subset has said which change they mean, and a message describing
/// more than they are about to commit would be wrong. The unstaged branch is
/// where the caller has expressed nothing, so it means "the work in this
/// tree" — which includes files that did not exist before.
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
    let whole = whole_tree_diff(dir)?;
    if !whole.trim().is_empty() {
        return Ok(Some((clipped(whole), false)));
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

/// Create the commit. A staged proposal commits exactly what is staged; an
/// unstaged one commits everything the proposal described. Returns git's
/// one-line summary.
///
/// It ran `git commit -am`, which stages tracked modifications and nothing
/// else — so a new file the message had just described (once the message
/// could see it at all) stayed uncommitted while the pane reported a
/// successful commit. `-A` with the same `.crew` exclusion the diff used
/// commits exactly what the user was shown, and nothing they were not.
pub(crate) fn do_commit(dir: &Path, message: &str, staged: bool) -> Result<String, String> {
    if !staged {
        git(dir, &["add", "-A", "--", super::changed::NOT_CREW])?;
    }
    let out = git(dir, &["commit", "-m", message])?;
    Ok(out.lines().next().unwrap_or("committed").to_string())
}

/// The commit capability (retired `/commit`, now reached through the intent
/// router): `rest` empty proposes a message for the current diff; `"apply"` —
/// sent only by the router's deterministic confirm gate — creates the commit
/// from the stored proposal.
pub(crate) fn commit_cmd(
    session: &mut Session,
    rest: &str,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let dir = match project_dir() {
        Ok(d) => d,
        Err(e) => return emit(msg("agent smith", format!("commit: {e}"))),
    };
    if rest.trim() == "apply" {
        let pending = lock(&session.commit).take();
        let Some(p) = pending else {
            return emit(msg(
                "agent smith",
                "no proposal — ask me to draft a commit message first",
            ));
        };
        let m = match do_commit(&dir, &p.message, p.staged) {
            Ok(s) => s,
            Err(e) => format!("commit failed: {e}"),
        };
        return emit(msg("agent smith", m));
    }
    let (diff, staged) = match pick_diff(&dir) {
        Err(e) => return emit(msg("agent smith", format!("commit: {e}"))),
        Ok(None) => {
            return emit(msg(
                "agent smith",
                "nothing to commit — the tree is clean (stage or edit something)",
            ))
        }
        Ok(Some(d)) => d,
    };
    let reg = session.registry();
    if reg.is_empty() {
        return emit(msg("agent smith", roster(&reg)));
    }
    // Elected by the agent's OWN role (`is_writer`), not the literal name
    // "coder" — see `review.rs`'s identical fix and `constructs::pick_judge`.
    let author = pick_by_role(&reg.infos(), is_writer);
    emit(msg(
        "agent smith",
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
        Some(Err(e)) => return emit(msg("agent smith", format!("commit draft failed: {e}"))),
        None => {
            return emit(msg(
                "agent smith",
                "commit stopped — the coder went missing",
            ))
        }
    };
    if message.is_empty() {
        return emit(msg("agent smith", "the draft came back empty — try again"));
    }
    emit(msg(&format!("{author} → user"), message.clone()))?;
    *lock(&session.commit) = Some(PendingCommit { message, staged });
    emit(msg(
        "agent smith",
        "proposal ready — say \u{201c}apply\u{201d} to create the commit, or \
         ask again to re-draft",
    ))
}

#[cfg(test)]
#[path = "gitmsg_tests.rs"]
mod tests;

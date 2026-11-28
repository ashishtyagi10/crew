//! "open a PR" — the state and the process runners the pull-request path
//! shares. The draft (`gitprdraft`) reads and writes nothing; the apply
//! (`gitprapply`) is reached only by the exact confirm in `intent::gate`, the
//! same propose/apply split as the commit message (`gitmsg`).
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex};

use super::gitmsg::PendingCommit;
use super::session::Session;
use crew_hive::childproc::no_console_window;

/// A drafted pull request awaiting the user's conversational "apply".
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PendingPr {
    /// The branch the PR comes from (pushed by apply).
    pub branch: String,
    /// True when the draft was made on the base branch, so apply must first
    /// create `branch` from HEAD. The base itself is never pushed.
    pub create_branch: bool,
    pub base: String,
    /// The commit the title and body describe. Apply refuses if HEAD moved:
    /// a commit made after the draft would be pushed under a description
    /// written without it.
    pub head: String,
    pub title: String,
    pub body: String,
}

/// The session's pending pull request, shared like the pending commit.
pub(crate) type SharedPr = Arc<Mutex<Option<PendingPr>>>;

/// Whether a drafted pull request is waiting for the confirm.
pub(crate) fn pending(session: &Session) -> bool {
    session
        .pr
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .is_some()
}

/// Forget a drafted pull request (`/clear`, rewind, a new plan).
pub(crate) fn drop_pr(session: &Session) {
    session.pr.lock().unwrap_or_else(|e| e.into_inner()).take();
}

/// The confirm that opens a pull request: "apply" alone, never the looser
/// "yes" / "do it" the commit and plan accept — this one writes to a remote,
/// and a "yes" answering some other question must not.
pub(crate) fn opens_pr(task: &str) -> bool {
    let t = task.trim().trim_end_matches(['.', '!']).to_lowercase();
    matches!(
        t.as_str(),
        "apply" | "apply it" | "yes apply" | "yes, apply"
    )
}

/// Store a drafted commit, dropping any pending PR: one pending thing at a
/// time, so "apply" is never ambiguous.
pub(crate) fn hold_commit(session: &Session, c: PendingCommit) {
    session.pr.lock().unwrap_or_else(|e| e.into_inner()).take();
    *session.commit.lock().unwrap_or_else(|e| e.into_inner()) = Some(c);
}

/// Store a drafted PR, dropping any pending commit (see [`hold_commit`]).
pub(crate) fn hold_pr(session: &Session, p: PendingPr) {
    session
        .commit
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take();
    *session.pr.lock().unwrap_or_else(|e| e.into_inner()) = Some(p);
}

/// The `gh` binary: `CREW_GH`, else `gh` on the PATH. The test seam for a
/// fake gh that records its argv.
pub(crate) fn gh_bin() -> String {
    std::env::var("CREW_GH")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "gh".into())
}

/// Stderr (else stdout) folded to one short line.
fn one_line(out: &std::process::Output) -> String {
    let text = match String::from_utf8_lossy(&out.stderr).trim() {
        "" => String::from_utf8_lossy(&out.stdout).trim().to_string(),
        e => e.to_string(),
    };
    let line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    line.chars().take(300).collect()
}

fn run(cmd: &mut Command, dir: &Path, what: &str) -> Result<String, String> {
    let out = cmd
        .current_dir(dir)
        .output()
        .map_err(|e| format!("{what}: {e}"))?;
    if !out.status.success() {
        return Err(one_line(&out));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim_end().to_string())
}

/// Run `git args` in `dir`; a failure is git's stderr on one line.
pub(crate) fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    run(
        no_console_window(&mut Command::new("git")).args(args),
        dir,
        "git",
    )
}

/// Run `gh args` in `dir`; a failure is gh's stderr on one line.
pub(crate) fn gh(dir: &Path, args: &[&str]) -> Result<String, String> {
    run(
        no_console_window(&mut Command::new(gh_bin())).args(args),
        dir,
        "gh",
    )
}

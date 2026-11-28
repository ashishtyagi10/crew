//! What must hold before a pull request can be drafted or opened. Run at the
//! draft and AGAIN at apply, since the tree may have changed in between. Each
//! unmet condition is one status line, never a draft.
use std::path::Path;

use std::process::Command;

use super::gitpr::{gh_bin, git};
use crew_hive::childproc::no_console_window;

/// A repo that is ready: where it is and what the PR would compare.
#[derive(Debug, Clone)]
pub(crate) struct Ready {
    /// The current branch.
    pub branch: String,
    /// The remote's default branch.
    pub base: String,
    /// What `base` is compared as: `origin/<base>` when that ref exists.
    pub base_ref: String,
}

fn exists(dir: &Path, r: &str) -> bool {
    git(dir, &["rev-parse", "--verify", "--quiet", r]).is_ok()
}

/// The remote's default branch: `origin/HEAD`, else a `main` / `master` that
/// exists (remote first, then local).
fn default_base(dir: &Path) -> Option<String> {
    if let Ok(h) = git(
        dir,
        &["symbolic-ref", "--short", "refs/remotes/origin/HEAD"],
    ) {
        if let Some(b) = h.strip_prefix("origin/") {
            return Some(b.to_string());
        }
    }
    ["main", "master"]
        .into_iter()
        .find(|b| {
            exists(dir, &format!("refs/remotes/origin/{b}"))
                || exists(dir, &format!("refs/heads/{b}"))
        })
        .map(str::to_string)
}

/// Check every precondition, in order; `Err` is the status line to show.
pub(crate) fn ready(dir: &Path) -> Result<Ready, String> {
    if git(dir, &["rev-parse", "--is-inside-work-tree"]).is_err() {
        return Err("not a git repository — nothing to open a PR from".into());
    }
    let mut c = Command::new(gh_bin());
    let auth = no_console_window(&mut c)
        .args(["auth", "status"])
        .current_dir(dir)
        .output();
    match auth {
        Err(_) => return Err("gh (the GitHub CLI) is not installed — install it first".into()),
        Ok(o) if !o.status.success() => {
            return Err("gh is not signed in — run `gh auth login` first".into())
        }
        Ok(_) => {}
    }
    if git(dir, &["remote", "get-url", "origin"]).is_err() {
        return Err("no `origin` remote — add one before opening a PR".into());
    }
    let dirty = git(
        dir,
        &["status", "--porcelain", "--", ".", super::changed::NOT_CREW],
    )?;
    if !dirty.trim().is_empty() {
        return Err("uncommitted changes — commit first (say \u{201c}commit this\u{201d}), then ask for the PR".into());
    }
    let branch = git(dir, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    if branch == "HEAD" {
        return Err("detached HEAD — check out a branch first".into());
    }
    let Some(base) = default_base(dir) else {
        return Err("no default branch found (origin/HEAD, main or master)".into());
    };
    let remote = format!("origin/{base}");
    let base_ref = if exists(dir, &format!("refs/remotes/{remote}")) {
        remote
    } else {
        base.clone()
    };
    let ahead = git(dir, &["rev-list", "--count", &format!("{base_ref}..HEAD")])?;
    if ahead.trim() == "0" {
        return Err(format!("nothing ahead of {base} — there is no PR to open"));
    }
    Ok(Ready {
        branch,
        base,
        base_ref,
    })
}

/// The one-line log and diff a PR describes, from `base_ref` to HEAD.
pub(crate) fn range(dir: &Path, base_ref: &str) -> Result<(String, String), String> {
    let log = git(
        dir,
        &[
            "log",
            "--reverse",
            "--format=- %s%n%b",
            &format!("{base_ref}..HEAD"),
        ],
    )?;
    let diff = git(dir, &["diff", &format!("{base_ref}...HEAD")])?;
    Ok((log, diff))
}

/// A branch name not yet taken: `name`, else `name-2`, `name-3`…
pub(crate) fn free_branch(dir: &Path, name: &str) -> String {
    let mut n = 1;
    loop {
        let cand = if n == 1 {
            name.to_string()
        } else {
            format!("{name}-{n}")
        };
        if !exists(dir, &format!("refs/heads/{cand}")) {
            return cand;
        }
        n += 1;
    }
}

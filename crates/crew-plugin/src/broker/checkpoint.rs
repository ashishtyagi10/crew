//! Cline-style workspace checkpoints, taken AUTOMATICALLY: every task that
//! reaches a worker pins the working tree first as a hidden commit under
//! `refs/crew/` (see `stdio::auto_checkpoint`), `/checkpoints` lists them and
//! `/restore <n>` puts one back. Snapshots use a temporary index, so they
//! never touch HEAD, the user's index, or any branch — and the refs survive
//! broker restarts.
//!
//! There is no `/checkpoint` construct any more. Asking a user to predict
//! which task is the one worth protecting is asking them to be right in
//! advance; an identical tree writes no new ref, so the automatic ones stay
//! meaningful, and [`prune`] caps the space they can take.
use std::path::Path;
use std::process::Command;

use crate::PluginEvent;

use super::relay::msg;

const REF_SPACE: &str = "refs/crew/";
const SUBJECT_PREFIX: &str = "crew checkpoint: ";

/// Run `git <args>` in `dir` (with `index` as `GIT_INDEX_FILE` when given);
/// trimmed stdout on success, trimmed stderr on failure.
fn git(dir: &Path, args: &[&str], index: Option<&Path>) -> Result<String, String> {
    let mut cmd = Command::new("git");
    cmd.args(args)
        .current_dir(dir)
        .env("GIT_OPTIONAL_LOCKS", "0");
    if let Some(idx) = index {
        cmd.env("GIT_INDEX_FILE", idx);
    }
    let out = cmd.output().map_err(|e| format!("git: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

/// The working tree's content hash — tracked and untracked files, `.gitignore`
/// respected — computed through a throwaway index so HEAD, the user's index
/// and every branch are untouched.
///
/// Split out from [`snapshot`] so an automatic checkpoint can ask "has
/// anything actually changed?" before writing a ref. Two questions in a row
/// that touch no files produce the same tree, and a session that snapshotted
/// every task regardless would bury the real restore points under identical
/// ones.
fn worktree_tree(dir: &Path) -> Result<String, String> {
    git(dir, &["rev-parse", "--git-dir"], None).map_err(|_| "not a git repository".to_string())?;
    // pid + a process-wide counter: unique even for simultaneous snapshots
    // (a wall-clock stamp collided under parallel tests).
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let tmp = std::env::temp_dir().join(format!(
        "crew-ckpt-index-{}-{}",
        std::process::id(),
        SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
    ));
    let result = (|| {
        git(dir, &["add", "-A"], Some(&tmp))?;
        git(dir, &["write-tree"], Some(&tmp))
    })();
    let _ = std::fs::remove_file(&tmp);
    result
}

/// How many checkpoints to keep. Snapshots became automatic in v0.6.44, so
/// the ref space grows on its own now — an autonomous feature owns its own
/// cleanup rather than quietly filling a user's repo with refs.
const KEEP: usize = 25;

/// Snapshot `dir`'s working tree and pin it under `refs/crew/`. Returns the
/// short commit id.
pub(crate) fn snapshot(dir: &Path, label: &str) -> Result<String, String> {
    let tree = worktree_tree(dir)?;
    let result = (|| {
        let subject = format!("{SUBJECT_PREFIX}{label}");
        // HEAD is the parent when it exists (unborn branches snapshot fine).
        let sha = match git(dir, &["rev-parse", "--verify", "HEAD"], None) {
            Ok(head) => git(
                dir,
                &["commit-tree", &tree, "-p", &head, "-m", &subject],
                None,
            )?,
            Err(_) => git(dir, &["commit-tree", &tree, "-m", &subject], None)?,
        };
        let short = &sha[..sha.len().min(12)];
        // The ref NAME carries the order. Git commit timestamps have only
        // second resolution, and checkpoints are automatic now, so two can
        // easily share a second — `--sort=creatordate` then falls back to the
        // object id, which is creation order only by accident. `/restore <n>`
        // counts these ordinals, so "by accident" is not good enough.
        git(
            dir,
            &[
                "update-ref",
                &format!("{REF_SPACE}ckpt-{:06}-{short}", next_seq(dir)),
                &sha,
            ],
            None,
        )?;
        prune(dir, KEEP);
        Ok(short.to_string())
    })();
    result
}

/// Drop all but the newest `keep` checkpoints. Best-effort: a failure here
/// must never fail the snapshot that triggered it.
fn prune(dir: &Path, keep: usize) {
    let Ok(all) = refs(dir) else { return };
    let Some(excess) = all.len().checked_sub(keep) else {
        return;
    };
    for (refname, _, _) in all.iter().take(excess) {
        let _ = git(dir, &["update-ref", "-d", refname], None);
    }
}

/// Snapshot before a task, but only if the working tree differs from
/// `last_tree` — which this updates. `Ok(None)` means "nothing had changed,
/// nothing was written", which is the common case and must stay silent.
///
/// Errors are the caller's to ignore: not being in a git repository is a
/// perfectly normal way to run crew, and an automatic safety net that
/// announced its own absence on every task would be noise, not safety.
pub(crate) fn auto_snapshot(
    dir: &Path,
    label: &str,
    last_tree: &mut Option<String>,
) -> Result<Option<String>, String> {
    let tree = worktree_tree(dir)?;
    if last_tree.as_deref() == Some(tree.as_str()) {
        return Ok(None);
    }
    let short = snapshot(dir, label)?;
    *last_tree = Some(tree);
    Ok(Some(short))
}

/// The next ordering number, one past the highest already written. Derived
/// from the refs themselves rather than held in memory, so it survives broker
/// restarts and stays correct across two brokers sharing a repository.
fn next_seq(dir: &Path) -> u64 {
    git(
        dir,
        &["for-each-ref", "--format=%(refname:lstrip=2)", REF_SPACE],
        None,
    )
    .map(|out| {
        out.lines()
            .filter_map(|n| n.strip_prefix("ckpt-"))
            .filter_map(|rest| rest.split_once('-'))
            .filter_map(|(seq, _)| seq.parse::<u64>().ok())
            .max()
            .map_or(0, |m| m + 1)
    })
    .unwrap_or(0)
}

/// Every checkpoint ref, oldest first, as `(full refname, short id, label)`.
///
/// `refname` matters: it is the only safe handle for deletion. Commit ids are
/// created here at 12 characters but `%(objectname:short)` abbreviates to
/// git's default (often 7), so rebuilding a ref name from a listed id deletes
/// nothing — silently, since `update-ref -d` on an absent ref succeeds.
///
/// The sort is by refname, not by date, because the name carries a zero-padded
/// creation sequence (see [`next_seq`]) and git commit dates do not have the
/// resolution to order automatic checkpoints. Refs written before v0.6.44 have
/// no sequence and sort among themselves by object id — historical ones may
/// therefore interleave, but every checkpoint taken from here on is ordered
/// exactly as it was created.
fn refs(dir: &Path) -> Result<Vec<(String, String, String)>, String> {
    let out = git(
        dir,
        &[
            "for-each-ref",
            "--sort=refname",
            "--format=%(refname)\t%(objectname:short)\t%(contents:subject)",
            REF_SPACE,
        ],
        None,
    )?;
    Ok(out
        .lines()
        .filter_map(|l| {
            let mut parts = l.splitn(3, '\t');
            Some((
                parts.next()?.to_string(),
                parts.next()?.to_string(),
                parts.next()?.to_string(),
            ))
        })
        .map(|(refname, sha, subject)| {
            let label = subject.strip_prefix(SUBJECT_PREFIX).unwrap_or(&subject);
            (refname, sha, label.to_string())
        })
        .collect())
}

/// The saved checkpoints, oldest first, as `(short id, label)`.
pub(crate) fn list(dir: &Path) -> Result<Vec<(String, String)>, String> {
    Ok(refs(dir)?
        .into_iter()
        .map(|(_, sha, label)| (sha, label))
        .collect())
}

/// Put checkpoint `sha`'s files back into the working tree. Only the worktree
/// changes — files created after the snapshot are left in place.
pub(crate) fn restore(dir: &Path, sha: &str) -> Result<(), String> {
    git(
        dir,
        &["restore", "--source", sha, "--worktree", "--", ":/"],
        None,
    )
    .map(|_| ())
}

/// `/checkpoints` — list the saved snapshots.
pub(crate) fn list_cmd(
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let dir = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => return emit(msg("agent smith", format!("checkpoints failed: {e}"))),
    };
    match list(&dir) {
        Ok(items) if items.is_empty() => emit(msg(
            "agent smith",
            // Checkpoints are automatic since v0.6.44; there is no
            // /checkpoint to point at any more.
            "no checkpoints yet \u{2014} one is taken before every task that \
             changes files, in a git repository",
        )),
        Ok(items) => {
            let lines: Vec<String> = items
                .iter()
                .enumerate()
                .map(|(i, (sha, label))| format!("#{} \u{00b7} {sha} \u{00b7} {label}", i + 1))
                .collect();
            emit(msg(
                "agent smith",
                format!(
                    "checkpoints (put one back with /restore <n>):\n{}",
                    lines.join("\n")
                ),
            ))
        }
        Err(e) => emit(msg("agent smith", format!("checkpoints failed: {e}"))),
    }
}

/// `/restore <n>` — put checkpoint `n`'s files back into the working tree.
/// `/restore` — bare, it LISTS the snapshots; with an ordinal it puts one
/// back. Two constructs for one subject is one too many: `/checkpoints` did
/// nothing `/restore` could not say for itself, exactly as `/agents` did
/// nothing bare `/model` could not.
pub(crate) fn restore_cmd(
    rest: &str,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    if rest.trim().is_empty() {
        return list_cmd(emit);
    }
    let dir = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => return emit(msg("agent smith", format!("restore failed: {e}"))),
    };
    let items = match list(&dir) {
        Ok(items) => items,
        Err(e) => return emit(msg("agent smith", format!("restore failed: {e}"))),
    };
    let n: Option<usize> = rest.trim().parse().ok();
    let Some((sha, label)) = n.and_then(|n| n.checked_sub(1)).and_then(|i| items.get(i)) else {
        return emit(msg(
            "agent smith",
            format!(
                "usage: /restore <1-{}> \u{2014} bare /restore lists them",
                items.len().max(1)
            ),
        ));
    };
    match restore(&dir, sha) {
        Ok(()) => emit(msg(
            "agent smith",
            format!(
                "restored \u{201c}{label}\u{201d} ({sha}) \u{2014} snapshot files are back; \
                 files created since the snapshot were left in place"
            ),
        )),
        Err(e) => emit(msg("agent smith", format!("restore failed: {e}"))),
    }
}

#[cfg(test)]
#[path = "checkpoint_tests.rs"]
mod tests;

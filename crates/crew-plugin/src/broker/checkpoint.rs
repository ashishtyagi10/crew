//! Cline-style workspace checkpoints: `/checkpoint [label]` snapshots the
//! working tree as a hidden commit pinned under `refs/crew/`, `/checkpoints`
//! lists them, and `/restore <n>` puts a snapshot's files back. Snapshots use
//! a temporary index, so they never touch HEAD, the user's index, or any
//! branch — and the refs survive broker restarts.
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

/// Snapshot `dir`'s working tree — tracked and untracked files, `.gitignore`
/// respected — and pin it under `refs/crew/`. Returns the short commit id.
pub(crate) fn snapshot(dir: &Path, label: &str) -> Result<String, String> {
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
        let tree = git(dir, &["write-tree"], Some(&tmp))?;
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
        git(
            dir,
            &["update-ref", &format!("{REF_SPACE}ckpt-{short}"), &sha],
            None,
        )?;
        Ok(short.to_string())
    })();
    let _ = std::fs::remove_file(&tmp);
    result
}

/// The saved checkpoints, oldest first, as `(short id, label)`.
pub(crate) fn list(dir: &Path) -> Result<Vec<(String, String)>, String> {
    let out = git(
        dir,
        &[
            "for-each-ref",
            "--sort=creatordate",
            "--format=%(objectname:short)\t%(contents:subject)",
            REF_SPACE,
        ],
        None,
    )?;
    Ok(out
        .lines()
        .filter_map(|l| l.split_once('\t'))
        .map(|(sha, subject)| {
            let label = subject.strip_prefix(SUBJECT_PREFIX).unwrap_or(subject);
            (sha.to_string(), label.to_string())
        })
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

/// `/checkpoint [label]` — snapshot the working directory.
pub(crate) fn checkpoint_cmd(
    rest: &str,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let label = Some(rest.trim())
        .filter(|l| !l.is_empty())
        .unwrap_or("checkpoint");
    let dir = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => return emit(msg("crew", format!("checkpoint failed: {e}"))),
    };
    match snapshot(&dir, label) {
        Ok(short) => {
            let n = list(&dir).map(|l| l.len()).unwrap_or(0);
            emit(msg(
                "crew",
                format!(
                    "checkpoint #{n} saved \u{00b7} {short} \u{00b7} \u{201c}{label}\u{201d} \
                     \u{2014} /restore {n} brings these files back"
                ),
            ))
        }
        Err(e) => emit(msg("crew", format!("checkpoint failed: {e}"))),
    }
}

/// `/checkpoints` — list the saved snapshots.
pub(crate) fn list_cmd(
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let dir = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => return emit(msg("crew", format!("checkpoints failed: {e}"))),
    };
    match list(&dir) {
        Ok(items) if items.is_empty() => emit(msg(
            "crew",
            "no checkpoints yet \u{2014} save one with /checkpoint [label]",
        )),
        Ok(items) => {
            let lines: Vec<String> = items
                .iter()
                .enumerate()
                .map(|(i, (sha, label))| format!("#{} \u{00b7} {sha} \u{00b7} {label}", i + 1))
                .collect();
            emit(msg(
                "crew",
                format!(
                    "checkpoints (restore with /restore <n>):\n{}",
                    lines.join("\n")
                ),
            ))
        }
        Err(e) => emit(msg("crew", format!("checkpoints failed: {e}"))),
    }
}

/// `/restore <n>` — put checkpoint `n`'s files back into the working tree.
pub(crate) fn restore_cmd(
    rest: &str,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let dir = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => return emit(msg("crew", format!("restore failed: {e}"))),
    };
    let items = match list(&dir) {
        Ok(items) => items,
        Err(e) => return emit(msg("crew", format!("restore failed: {e}"))),
    };
    let n: Option<usize> = rest.trim().parse().ok();
    let Some((sha, label)) = n.and_then(|n| n.checked_sub(1)).and_then(|i| items.get(i)) else {
        return emit(msg(
            "crew",
            format!(
                "usage: /restore <1-{}> \u{2014} see /checkpoints",
                items.len().max(1)
            ),
        ));
    };
    match restore(&dir, sha) {
        Ok(()) => emit(msg(
            "crew",
            format!(
                "restored \u{201c}{label}\u{201d} ({sha}) \u{2014} snapshot files are back; \
                 files created since the snapshot were left in place"
            ),
        )),
        Err(e) => emit(msg("crew", format!("restore failed: {e}"))),
    }
}

#[cfg(test)]
#[path = "checkpoint_tests.rs"]
mod tests;

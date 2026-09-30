//! Which files `sys:grep` and `sys:glob` look at.
//!
//! The fixed skip list knows the build and dependency directories every
//! project of a kind shares. What one project generates or vendors besides —
//! `.venv`, `coverage/`, `*.min.js`, logs, checked-out SDKs — it names in its
//! `.gitignore`, and hits there crowded out the ones in its own code. Inside a
//! git work tree the list is git's: tracked files and untracked ones that are
//! not ignored, which is ripgrep's rule and the one agents expect. Outside
//! one, or when git fails, it is the walk with the fixed list, as before.
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use crew_hive::childproc::no_console_window;

/// Directories the walk never descends into.
const SKIP_DIRS: &[&str] = &["target", "node_modules", "dist", "build", "__pycache__"];

/// `ls-files` lists a large monorepo in well under a second; past this the
/// walk answers instead of the search waiting on git.
const LIST_TIMEOUT: Duration = Duration::from_secs(10);

/// Bytes of listing read. A tree with more than 64 MB of path names is walked
/// instead, rather than searched through a list with its end missing.
const LIST_CAP: usize = 64 << 20;

/// The files under `root` to search, each as its path relative to `root`
/// (`/`-separated, the way hits name it) and the path to open, in the walk's
/// order: depth first, each directory's entries by name.
pub(super) fn files(root: &Path) -> Vec<(String, PathBuf)> {
    match listed(root) {
        // The fixed list still holds here: a `target/` or `node_modules/`
        // the project forgot to ignore is still never the answer.
        Some(rels) => rels
            .into_iter()
            .filter(|r| !r.split('/').any(|c| SKIP_DIRS.contains(&c)))
            .map(|r| (r.clone(), root.join(r)))
            .collect(),
        None => walked(root),
    }
}

/// Git's list of what under `root` is the project's: tracked, or untracked
/// and not ignored. `None` outside a work tree, when git fails or runs past
/// [`LIST_TIMEOUT`] — and when it lists nothing, because a directory that is
/// ignored as a whole (`target/debug`) is only ever passed on purpose, and
/// the agent that names it wants it searched.
fn listed(root: &Path) -> Option<Vec<String>> {
    if !root.is_dir() {
        return None;
    }
    let mut cmd = Command::new("git");
    no_console_window(&mut cmd);
    cmd.args([
        "ls-files",
        "-z",
        "--cached",
        "--others",
        "--exclude-standard",
    ])
    .current_dir(root)
    .env("GIT_OPTIONAL_LOCKS", "0")
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::null());
    let mut child = cmd.spawn().ok()?;
    let deadline = Instant::now() + LIST_TIMEOUT;
    // Git closes its stdout as it exits, so the listing arriving whole is the
    // sign it is done; the wait after only collects its exit status.
    let listing = super::sysgit::drain(child.stdout.take(), LIST_CAP).recv_timeout(LIST_TIMEOUT);
    let Ok((text, cut)) = listing else {
        return stopped(child);
    };
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(1)),
            _ => return stopped(child),
        }
    };
    if !status.success() || cut {
        return None;
    }
    let mut rels: Vec<String> = text
        .split('\0')
        .filter(|r| !r.is_empty())
        .map(str::to_string)
        .collect();
    // By component, so `a/x` comes before `a-b/x` as it does in the walk —
    // a whole-path sort puts `-` before `/`. An unmerged file is listed once
    // per stage.
    rels.sort_by(|a, b| a.split('/').cmp(b.split('/')));
    rels.dedup();
    (!rels.is_empty()).then_some(rels)
}

/// Git that ran past the deadline, or whose listing never came: stopped, so
/// the walk answers instead.
fn stopped(mut child: Child) -> Option<Vec<String>> {
    let _ = child.kill();
    let _ = child.wait();
    None
}

/// Every file under `root` the walk reaches: hidden entries and
/// [`SKIP_DIRS`] are passed over, and so are the rules in any `.gitignore`.
fn walked(root: &Path) -> Vec<(String, PathBuf)> {
    walkdir::WalkDir::new(root)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(visible)
        .flatten()
        .filter(|e| e.file_type().is_file())
        .map(|e| (rel_of(root, e.path()), e.into_path()))
        .collect()
}

/// Whether the walk should enter/consider `e`.
fn visible(e: &walkdir::DirEntry) -> bool {
    let name = e.file_name().to_string_lossy();
    if e.depth() == 0 {
        return true;
    }
    if name.starts_with('.') {
        return false;
    }
    !(e.file_type().is_dir() && SKIP_DIRS.contains(&name.as_ref()))
}

/// `p` relative to `root`; a FILE passed as the root (a saved output from
/// `spill`, say) is named as it was passed, since stripping it from itself
/// leaves hits reading `:412: …`, with no file for `sys:read_file` to open.
fn rel_of(root: &Path, p: &Path) -> String {
    let rel = p.strip_prefix(root).unwrap_or(p);
    let rel = if rel.as_os_str().is_empty() { p } else { rel };
    rel.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
#[path = "sysgrepfiles_tests.rs"]
mod tests;

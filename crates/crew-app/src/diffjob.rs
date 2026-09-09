//! `/diff`'s background read of the working tree.
//!
//! The diff itself is rendered by the file viewer's diff rung — pairing,
//! word-level marks, hunk headings — rather than by `git`'s own colours in a
//! scrollback, so what `/diff` needs is the *text*: `git status --short`, the
//! stat, the full unified diff, and then every UNTRACKED file as an addition
//! against nothing, concatenated and dropped in a temp file the viewer can
//! open. The last part exists because `git diff` is index-relative: a file an
//! agent has just created — the most likely thing in a tree after an agent
//! ran — was a `??` line in the status and absent from the review itself.
//!
//! It runs on a worker thread. `git diff` takes seconds on a large or
//! network-mounted repo, and every pane in the grid — agents included — is
//! frozen for as long as anything blocks the winit thread.
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{self, Receiver};

use crew_hive::childproc::no_console_window;

/// The three reads that make up a review, in the order they are stacked.
const PARTS: [&[&str]; 3] = [
    &["status", "--short"],
    &["--no-pager", "diff", "--stat"],
    &["--no-pager", "diff"],
];

/// The file a repo's diff is written to. Named after the directory so
/// re-running `/diff` in one repo overwrites rather than litters, and two
/// repos never collide.
pub(crate) fn temp_path(dir: &Path) -> PathBuf {
    let slug: String = dir
        .to_string_lossy()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    // Long paths make long names; the tail is the part that differs.
    let tail: String = slug
        .chars()
        .rev()
        .take(60)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    std::env::temp_dir().join(format!("crew-diff{tail}.diff"))
}

/// Run `git <args>` in `dir`; stdout when the exit code is one of `ok`,
/// stderr otherwise. `diff --no-index` exits 1 to mean "differs", which for a
/// review is the answer, not a failure.
fn git(dir: &Path, args: &[&str], ok: &[i32]) -> Result<String, String> {
    let mut cmd = Command::new("git");
    no_console_window(&mut cmd);
    cmd.args(args).current_dir(dir);
    let r = cmd.output().map_err(|e| format!("git: {e}"))?;
    if !r.status.code().is_some_and(|c| ok.contains(&c)) {
        return Err(String::from_utf8_lossy(&r.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&r.stdout).into_owned())
}

/// The text of a review of `dir`, or the reason there is none.
fn read(dir: &Path) -> Result<String, String> {
    let mut out = String::new();
    for args in PARTS {
        out.push_str(&git(dir, args, &[0])?);
    }
    // Untracked files, `.gitignore` respected, each as a diff against
    // nothing — the shape the viewer's diff rung already reads.
    let fresh = git(dir, &["ls-files", "--others", "--exclude-standard"], &[0])?;
    for path in fresh.lines().filter(|p| !p.is_empty()) {
        let args = ["--no-pager", "diff", "--no-index", "--", "/dev/null", path];
        out.push_str(&git(dir, &args, &[0, 1])?);
    }
    Ok(out)
}

/// The one `/diff` read allowed to be in flight.
#[derive(Default)]
pub(crate) struct DiffJob {
    rx: Option<Receiver<Result<PathBuf, String>>>,
}

impl DiffJob {
    /// Whether a read is already running — a second `/diff` while the first
    /// is still going would race it to the same file.
    pub(crate) fn busy(&self) -> bool {
        self.rx.is_some()
    }

    /// Start reading `dir`'s working tree.
    pub(crate) fn start(&mut self, dir: PathBuf) {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let path = temp_path(&dir);
            let done = read(&dir).and_then(|text| match text.trim().is_empty() {
                true => Err("nothing to review — the working tree is clean".into()),
                false => std::fs::write(&path, text)
                    .map(|()| path)
                    .map_err(|e| format!("cannot write the review: {e}")),
            });
            let _ = tx.send(done);
        });
        self.rx = Some(rx);
    }

    /// The finished read, once. `None` while it is still running.
    pub(crate) fn take(&mut self) -> Option<Result<PathBuf, String>> {
        let done = match self.rx.as_ref()?.try_recv() {
            Ok(done) => done,
            Err(mpsc::TryRecvError::Empty) => return None,
            Err(mpsc::TryRecvError::Disconnected) => Err("the review was interrupted".into()),
        };
        self.rx = None;
        Some(done)
    }
}

#[cfg(test)]
#[path = "diffjob_tests.rs"]
mod tests;

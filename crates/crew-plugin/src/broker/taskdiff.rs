//! The diff a task made, in the pane — not just the names of the files.
//!
//! `changed::summary` says WHICH files a task touched; the user still had to
//! run `/diff` and read it somewhere else to learn WHAT, which is the question
//! a change actually raises. The pane already renders a fenced ```diff block
//! — added lines green, removed red, hunk headers cyan, the changed words
//! marked — and nothing ever sent it one. This module sends it one, then asks
//! the language servers what they make of the files it touched, and says that
//! in a second, short message of its own.
//!
//! Two messages, because the diff is ready long before the verdict can be.
//! They used to be one body, built after the diagnostics came back, and on a
//! freshly started rust-analyzer that meant the file summary at +42.5 s and
//! then nothing until +54.9 s, when the diff and the verdict arrived together:
//! the diff had been ready for twelve seconds, waiting on the index. Now the
//! diff goes out the moment it is computed and the verdict follows it.
//!
//! Both halves are bounded. The patch is clipped at [`PATCH_CAP`] on a line
//! boundary (`/diff` has the rest), and the diagnostics stop at [`DIAG_BUDGET`]
//! (the deadline lives in `taskdiag`): this runs on the task worker after the
//! reply has streamed, and a user kept waiting on a language server would
//! blame the task. No server is not an annotation either — the diff must read
//! perfectly with no LSP on the machine at all.
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::changed::Change;

/// How much of the patch the note carries, in chars. The commit-message
/// prompt's budget, for the same reason: past it a reader wants the file.
pub(crate) const PATCH_CAP: usize = 12_000;
/// Total time the diagnostics pass may take across every changed file — a
/// deadline, measured from the first ask (`taskdiag`).
pub(crate) const DIAG_BUDGET: Duration = Duration::from_secs(8);
/// Diagnostics named before the rest are counted.
const DIAG_LINES: usize = 20;

/// What the language servers said about the changed files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Diag {
    /// Nothing on this machine serves any of the files: say nothing.
    NoServer,
    /// A server looked at N files and found nothing wrong.
    Clean(usize),
    /// One line per diagnostic, the way the `lsp` tool prints them.
    Lines(Vec<String>),
}

/// `patch` cut on a line boundary once it is longer than `cap` chars, with a
/// note counting the lines that were dropped; `hint` is appended to the note
/// (where the rest can be read). Whole when it fits.
pub(crate) fn clip(patch: &str, cap: usize, hint: &str) -> String {
    let total = patch.lines().count();
    let (mut kept, mut chars) = (0usize, 0usize);
    for line in patch.lines() {
        let n = line.chars().count() + usize::from(kept > 0);
        if chars + n > cap {
            break;
        }
        chars += n;
        kept += 1;
    }
    if kept == total {
        return patch.to_string();
    }
    let mut out: String = if kept == 0 {
        // The first line alone is wider than the cap — a minified file, most
        // likely. Its head says more than nothing would.
        kept = 1;
        patch
            .lines()
            .next()
            .unwrap_or("")
            .chars()
            .take(cap)
            .collect()
    } else {
        patch.lines().take(kept).collect::<Vec<_>>().join("\n")
    };
    out.push_str(&format!("\n\u{2026} (+{} more lines{hint})", total - kept));
    out
}

fn clip_patch(patch: &str) -> String {
    clip(patch, PATCH_CAP, " \u{2014} /diff shows the whole patch")
}

/// `patch` inside a ```diff fence the pane's diff lexer renders.
pub(crate) fn fenced(patch: &str) -> String {
    // A closing fence is any line of at least as many backticks as the
    // opening one, indented up to three spaces — and a diff's context lines
    // are indented exactly one. So the fence is one backtick longer than the
    // longest run that opens a line inside the patch. The `+`/`-` marker is
    // skipped too: a renderer that is lenient about it would break the card,
    // and a fence one backtick longer than it strictly needed costs nothing.
    let longest = patch
        .lines()
        .map(|l| {
            let body = l.trim_start().trim_start_matches(['+', '-']).trim_start();
            body.chars().take_while(|c| *c == '`').count()
        })
        .max()
        .unwrap_or(0);
    let fence = "`".repeat(longest.max(2) + 1);
    format!("{fence}diff\n{patch}\n{fence}")
}

/// Emit what a finished task changed: the diff at once, then — after asking
/// `diagnostics` about the files that still exist — the verdict. Nothing when
/// there were no changes or git cannot produce the patch. The files are
/// asked about as absolute paths (a deleted file has nothing left to
/// diagnose), since git names them from the repository root and the worker's
/// cwd may be below it.
pub(crate) fn report(
    dir: &Path,
    base: &str,
    changes: &[Change],
    diagnostics: impl FnOnce(&[String]) -> Diag,
    emit: &mut dyn FnMut(String),
) {
    if changes.is_empty() {
        return;
    }
    let Ok(patch) = super::changed::patch(dir, base) else {
        return;
    };
    let root = super::checkpoint::git(dir, &["rev-parse", "--show-toplevel"], None)
        .map_or_else(|_| dir.to_path_buf(), PathBuf::from);
    let files: Vec<String> = changes
        .iter()
        .filter(|(status, _)| *status != 'D')
        .map(|(_, path)| root.join(path).to_string_lossy().into_owned())
        .collect();
    deliver(&patch, &files, diagnostics, emit);
}

/// The two messages, in the order that keeps the diff off the server's clock:
/// the diff is emitted BEFORE `diagnostics` is asked anything. Split from
/// [`report`] so the order is tested without a repository.
pub(crate) fn deliver(
    patch: &str,
    files: &[String],
    diagnostics: impl FnOnce(&[String]) -> Diag,
    emit: &mut dyn FnMut(String),
) {
    if let Some(diff) = diff_note(patch) {
        emit(diff);
    }
    if let Some(said) = verdict(diagnostics(files)) {
        emit(said);
    }
}

/// The patch, clipped and fenced; `None` when there is nothing to show.
fn diff_note(patch: &str) -> Option<String> {
    (!patch.trim().is_empty()).then(|| fenced(&clip_patch(patch)))
}

/// What the language servers said, in a line or a list; `None` for silence.
fn verdict(diag: Diag) -> Option<String> {
    match diag {
        Diag::NoServer => None,
        Diag::Clean(1) => Some("no diagnostics in the changed file".to_string()),
        Diag::Clean(n) => Some(format!("no diagnostics in the {n} changed files")),
        Diag::Lines(lines) => {
            let mut s = String::from("diagnostics after the change:");
            for line in lines.iter().take(DIAG_LINES) {
                s.push('\n');
                s.push_str(line);
            }
            if let Some(rest) = lines.len().checked_sub(DIAG_LINES).filter(|n| *n > 0) {
                s.push_str(&format!("\n\u{2026} +{rest} more"));
            }
            Some(s)
        }
    }
}

#[cfg(test)]
#[path = "taskdiff_tests.rs"]
mod tests;

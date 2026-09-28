//! An edit that broke the build says so in its own result.
//!
//! The language servers' word on the files a task changed came only when the
//! task ended (`taskdiag`), after the agent had already answered "done".
//! Mid-task, an agent that had just written a type error found out only if it
//! thought to call `lsp:diagnostics`, which models rarely do. Claude Code hands
//! the new diagnostics back with the edit, and the model fixes its own error
//! in the next round; so does crew now. A `sys:edit` or `sys:write_file` that
//! succeeded on a file a running language server serves ends with the errors
//! the server found in it that it had not reported before the write:
//!
//! ```text
//! diagnostics now: 2 new errors
//!   12:5 error: mismatched types — expected `i32`, found `&str`
//! ```
//!
//! New, not every error in the file, like Claude Code's: one the server had
//! already reported is either its word on the old text (rust-analyzer goes on
//! publishing `cargo check`'s word on the old text until the check the save
//! started finishes, so a fix read as the error it fixed, measured) or was
//! there before this write and is not its doing; `lsp:diagnostics` and the
//! task's end list every one. See `LspHost::new_diagnostics`.
//!
//! Errors only. Warnings (an import not used yet halfway through a change, a
//! variable not read yet) are true, are the next edit's business, and would
//! drown the one line that says the build is broken. No errors, no note: a
//! "clean" after every edit is context spent on nothing.
//!
//! Bounded, because the model is waiting on the edit: the ask runs on a
//! thread of its own (the shape `taskdiag` uses), the edit stops listening at
//! [`BOUND`], and the host is handed the time left so it lets go of its lock
//! by then too. A server not running yet (the task's first write is starting
//! it), one still indexing, one that has not spoken about the new text in
//! time: all silence, and the task's end still reports.
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use crew_lsp::{Diagnostic, Severity};

/// The most the diagnostics add to an edit.
const BOUND: Duration = Duration::from_secs(2);
/// Errors listed; the header's count covers the rest.
const SHOWN: usize = 5;
/// Chars of one error's line: a trait-bound error runs to a screenful, and
/// the first of it says what broke.
const LINE_CAP: usize = 200;

/// Where an edit's diagnostics come from — the seam that lets the bound and
/// the filter run against a fake, since a language server cannot be assumed
/// on the machine running the tests.
pub(crate) trait Source {
    /// Whether a server for `path`'s language is configured and installed.
    fn serves(&self, path: &Path) -> bool;
    /// What a server already running for `path` has said about it since the
    /// write that it had not said before, heard within `wait`; `None` when no
    /// server is running for it yet.
    fn diagnostics(&mut self, path: &Path, wait: Duration) -> Option<Vec<Diagnostic>>;
}

/// The source as a session's tool surface holds it.
pub(crate) type Shared = Arc<Mutex<dyn Source + Send>>;

impl Source for crate::lsp::LspHost {
    fn serves(&self, path: &Path) -> bool {
        self.serves_file(path)
    }
    fn diagnostics(&mut self, path: &Path, wait: Duration) -> Option<Vec<Diagnostic>> {
        self.new_diagnostics(path, wait)
    }
}

/// `out`, what a `sys` call that succeeded answered, with the new errors in
/// the file it wrote after it when it wrote one and there are any.
pub(crate) fn append(out: String, src: &Shared, tool: &str, args: &str) -> String {
    match written(tool, args).and_then(|path| errors(src, path)) {
        Some(block) => format!("{out}\n\n{block}"),
        None => out,
    }
}

/// The file a write named, when it is in a language some server could serve:
/// anything else (a README, a `sys:run`) costs no thread and no lock.
fn written(tool: &str, args: &str) -> Option<PathBuf> {
    if !matches!(tool, "edit" | "write_file") {
        return None;
    }
    super::lspwarm::path_arg(args).filter(|p| crew_lsp::servers::lang_of(p).is_some())
}

/// The note for `path`, asked on a thread of its own and given up at
/// [`BOUND`], wherever the host is stuck: on its lock, or on the server.
fn errors(src: &Shared, path: PathBuf) -> Option<String> {
    let deadline = Instant::now() + BOUND;
    let (tx, rx) = mpsc::channel();
    let src = Arc::clone(src);
    std::thread::spawn(move || {
        let mut s = src.lock().unwrap_or_else(|e| e.into_inner());
        // The lock came free too late: nobody is listening any more, and an
        // ask now would only hold it against the next question.
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() || !s.serves(&path) {
            return;
        }
        let _ = tx.send(s.diagnostics(&path, left));
    });
    let left = deadline.saturating_duration_since(Instant::now());
    block(&rx.recv_timeout(left).ok().flatten()?)
}

/// `diagnostics now: N new errors` and the first [`SHOWN`] in file order,
/// one line each; `None` when there are no errors.
///
/// The file is not named: it is the one the agent just wrote. The message's
/// lines are joined, since rust-analyzer puts rustc's "expected `i32`, found
/// `&str`" on the line after "mismatched types", and it is the half that says
/// what to change.
fn block(list: &[Diagnostic]) -> Option<String> {
    let mut errors: Vec<&Diagnostic> = list
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    if errors.is_empty() {
        return None;
    }
    errors.sort_by_key(|d| (d.range.start.line, d.range.start.character));
    let n = errors.len();
    let plural = if n == 1 { "" } else { "s" };
    let mut lines = vec![format!("diagnostics now: {n} new error{plural}")];
    lines.extend(errors.iter().take(SHOWN).map(|d| line(d)));
    if n > SHOWN {
        lines.push(format!("  \u{2026} {} more", n - SHOWN));
    }
    Some(lines.join("\n"))
}

/// `  12:5 error: message`, one-based like a compiler prints it.
fn line(d: &Diagnostic) -> String {
    let message: Vec<&str> = d
        .message
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    let text = format!(
        "  {}:{} error: {}",
        d.range.start.line + 1,
        d.range.start.character + 1,
        message.join(" \u{2014} ")
    );
    match text.char_indices().nth(LINE_CAP) {
        Some((at, _)) => format!("{}\u{2026}", &text[..at]),
        None => text,
    }
}

#[cfg(test)]
#[path = "editdiag_tests.rs"]
mod tests;

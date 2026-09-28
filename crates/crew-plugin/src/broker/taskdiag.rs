//! What the language servers make of the files a task just changed — the
//! second half of `taskdiff`, on its own because it has its own seam (a fake
//! host, since a language server cannot be assumed on the machine running the
//! tests) and its own bound (the budget).
//!
//! The budget is a deadline, not a check. It used to be read only BEFORE each
//! file's ask, so the first ask on a freshly started server blocked for as
//! long as the server took to index: measured on the crew repo, the file
//! summary landed at +42.5 s and the diff and its verdict at +54.9 s, twelve
//! seconds of rust-analyzer's cold start on an 8 s budget. The asks now run on
//! a thread of their own and the caller stops listening when the budget is
//! spent, wherever the host is stuck: waiting on the lock, starting, or
//! waiting for a publish.
//!
//! Silence is a first-class answer here. No server for a language, a server
//! that is not installed, a server that failed, a server not ready in time:
//! all say nothing, because a diff annotated with "could not check" on every
//! slow machine would teach users to ignore the annotation.
use std::path::Path;
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use super::taskdiff::Diag;

/// Where diagnostics come from — the seam that lets the budget loop run
/// against a fake, since a language server cannot be assumed on the machine
/// running the tests.
pub(crate) trait DiagSource {
    /// Whether anything on this machine serves `path`'s language.
    fn serves(&self, path: &str) -> bool;
    /// The `lsp` tool's `diagnostics` text for one file.
    fn diagnostics(&mut self, path: &str) -> Result<String, String>;
}

impl DiagSource for crate::lsp::LspHost {
    fn serves(&self, path: &str) -> bool {
        self.serves_file(Path::new(path))
    }
    fn diagnostics(&mut self, path: &str) -> Result<String, String> {
        self.call(
            "diagnostics",
            &serde_json::json!({ "file": path }).to_string(),
        )
    }
}

/// Ask `host` about each of `files` it serves, in order, and stop listening
/// once `budget` has been spent; what came back before then is reported.
///
/// An ask still in flight at the deadline is left to finish on its thread —
/// it holds the host's lock until the server answers or gives up, so a
/// question the model asks next may wait on it, but the task's ending does
/// not. The thread asks nothing after the deadline.
pub(crate) fn lsp_diagnostics<S>(host: &Arc<Mutex<S>>, files: &[String], budget: Duration) -> Diag
where
    S: DiagSource + Send + 'static,
{
    let deadline = Instant::now() + budget;
    let (tx, rx) = mpsc::channel();
    let (host, list) = (Arc::clone(host), files.to_vec());
    std::thread::spawn(move || {
        for file in list {
            if Instant::now() >= deadline {
                return;
            }
            let mut h = host.lock().unwrap_or_else(|e| e.into_inner());
            if !h.serves(&file) {
                continue;
            }
            let answer = h.diagnostics(&file);
            drop(h);
            // Nobody is listening past the deadline: stop asking.
            if tx.send(answer).is_err() {
                return;
            }
        }
    });
    let (mut answered, mut lines) = (0usize, Vec::new());
    // Out of time, or every file asked (the thread hung up): either way, done.
    while let Ok(answer) = rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
        // A server that failed is silence, not a report: "no diagnostics"
        // about a file nobody managed to look at would be a lie.
        let Ok(text) = answer else {
            continue;
        };
        answered += 1;
        lines.extend(
            text.lines()
                .filter(|l| !l.starts_with("no diagnostics for "))
                .map(str::to_string),
        );
    }
    match (answered, lines.is_empty()) {
        (0, _) => Diag::NoServer,
        (n, true) => Diag::Clean(n),
        _ => Diag::Lines(lines),
    }
}

#[cfg(test)]
#[path = "taskdiag_tests.rs"]
mod tests;

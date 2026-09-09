//! What the language servers make of the files a task just changed — the
//! second half of `taskdiff`, on its own because it has its own seam (a fake
//! host, since a language server cannot be assumed on the machine running the
//! tests) and its own bound (the budget).
//!
//! Silence is a first-class answer here. No server for a language, a server
//! that is not installed, a server that failed: all say nothing, because a
//! diff annotated with "could not check" on every machine without
//! rust-analyzer would teach users to ignore the annotation.
use std::path::Path;
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

/// Ask `host` about each of `files` it serves, in order, until `budget` has
/// been spent. One fresh open can take the whole budget by itself (a server
/// indexing a large project), so the check is before each ask rather than a
/// deadline on the whole; what came back before the stop is still reported.
pub(crate) fn lsp_diagnostics(
    host: &mut dyn DiagSource,
    files: &[String],
    budget: Duration,
) -> Diag {
    let started = Instant::now();
    let (mut answered, mut lines) = (0usize, Vec::new());
    for file in files {
        if !host.serves(file) {
            continue;
        }
        if started.elapsed() > budget {
            break;
        }
        // A server that failed is silence, not a report: "no diagnostics"
        // about a file nobody managed to look at would be a lie.
        let Ok(text) = host.diagnostics(file) else {
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

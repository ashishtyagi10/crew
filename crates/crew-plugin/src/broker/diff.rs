//! `/diff` — codex-style working-tree diff. Read-only and bounded, so it runs
//! inline as a quick construct (see `commands::handle`) the same way
//! `/checkpoint` shells out to git in `checkpoint.rs`.
use std::path::Path;
use std::process::Command;

use crate::PluginEvent;

use super::relay::msg;

/// Run `git diff --stat` in `dir`; raw stdout on success, trimmed stderr on
/// failure. `--stat` keeps the output compact even before `diff_report`'s cap.
fn git_diff_stat(dir: &Path) -> Result<String, String> {
    let out = Command::new("git")
        .args(["diff", "--stat"])
        .current_dir(dir)
        .output()
        .map_err(|e| format!("git: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

/// Format `git diff --stat` output for the crew pane. Empty (clean tree) →
/// a friendly line; long output is bounded so a huge repo can't flood the pane.
pub(crate) fn diff_report(raw_stat: &str) -> String {
    let trimmed = raw_stat.trim();
    if trimmed.is_empty() {
        return "working tree clean \u{2014} no changes".to_string();
    }
    const CAP: usize = 4000;
    if trimmed.len() > CAP {
        let mut s: String = trimmed.chars().take(CAP).collect();
        s.push_str("\n\u{2026} (diff truncated)");
        s
    } else {
        trimmed.to_string()
    }
}

/// `/diff` — show the working tree's changes (`git diff --stat`), bounded.
pub(crate) fn diff_cmd(
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let dir = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => return emit(msg("crew", format!("diff failed: {e}"))),
    };
    match git_diff_stat(&dir) {
        Ok(raw) => emit(msg("crew", diff_report(&raw))),
        Err(e) => emit(msg("crew", format!("diff failed: {e}"))),
    }
}

#[cfg(test)]
#[path = "diff_tests.rs"]
mod tests;

//! Persisting the input-bar command history across sessions.
use std::path::{Path, PathBuf};

/// Keep at most this many recent lines on disk.
const MAX: usize = 1000;

fn path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("crew").join("history"))
}

/// The last `MAX` lines, newline-joined (oldest first).
fn serialize(history: &[String]) -> String {
    let start = history.len().saturating_sub(MAX);
    history[start..].join("\n")
}

/// Parse stored history: non-empty lines, oldest first.
fn deserialize(s: &str) -> Vec<String> {
    s.lines()
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect()
}

/// Load the persisted command history (empty if none / unreadable).
pub fn load() -> Vec<String> {
    let Some(p) = path() else {
        return Vec::new();
    };
    std::fs::read_to_string(&p)
        .map(|s| deserialize(&s))
        .unwrap_or_default()
}

/// Persist the command history (capped to the most recent `MAX` lines).
pub fn save(history: &[String]) {
    let Some(p) = path() else {
        return;
    };
    save_to(&p, history);
}

/// [`save`] to an explicit path — the testable half, so no test ever writes
/// the user's real history file.
///
/// Owner-only (0600 on unix). This file holds every line typed into the input
/// bar, and a bar that has stolen focus from an open key prompt is exactly how
/// a secret ends up being typed into it — but the reason is broader than that
/// one bug: a shell history is a well-known place for a pasted token to be
/// captured, and this one is world-readable no more. Written through
/// `credentials::write_atomic`, which creates the file 0600 BEFORE any bytes
/// land in it and renames it into place; `std::fs::write` creates 0644 and can
/// only be chmod'd afterwards, leaving a window where anyone can read it.
fn save_to(p: &Path, history: &[String]) {
    if let Some(parent) = p.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = crew_plugin::credentials::write_atomic(p, serialize(history).as_bytes());
}

#[cfg(test)]
#[path = "history_tests.rs"]
mod tests;

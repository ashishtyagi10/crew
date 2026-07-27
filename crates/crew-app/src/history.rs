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
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_filters_blanks() {
        let h = vec!["ls".to_string(), "cargo test".to_string()];
        assert_eq!(deserialize(&serialize(&h)), h);
        assert_eq!(
            deserialize("a\n\n b \n"),
            vec!["a".to_string(), " b ".to_string()]
        );
    }

    /// The history file can capture anything a user typed into the input bar,
    /// including a secret typed into it by mistake. `std::fs::write` created
    /// it 0644 — world-readable — with no mode ever set.
    #[cfg(unix)]
    #[test]
    fn saved_history_is_owner_only_and_still_round_trips() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("history");
        save_to(&p, &["ls".to_string(), "cargo test".to_string()]);
        let mode = std::fs::metadata(&p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "history must not be readable by anyone else");
        assert_eq!(
            deserialize(&std::fs::read_to_string(&p).unwrap()),
            vec!["ls".to_string(), "cargo test".to_string()]
        );
        // Atomic write: no temp file left behind.
        assert!(!p.with_extension("tmp").exists());
    }

    /// Re-saving over an existing world-readable history (one written by an
    /// older build) must leave it owner-only, not inherit the old mode.
    #[cfg(unix)]
    #[test]
    fn saving_over_a_world_readable_history_tightens_it() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("history");
        std::fs::write(&p, "old\n").unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o644)).unwrap();
        save_to(&p, &["new".to_string()]);
        let mode = std::fs::metadata(&p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "new");
    }

    #[test]
    fn serialize_caps_to_max() {
        let h: Vec<String> = (0..MAX + 50).map(|i| i.to_string()).collect();
        let out = deserialize(&serialize(&h));
        assert_eq!(out.len(), MAX);
        assert_eq!(out.first().unwrap(), "50"); // oldest 50 dropped
    }
}

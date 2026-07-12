//! Command history for the Far command bar: persisted beside the existing
//! chat-input history (same `dirs` base as `crate::history`, a sibling file
//! named `far-history`), newline-delimited, deduped against the immediately
//! preceding entry, capped at 500 entries (oldest dropped), loaded once per
//! pane. Also serves fish-style ghost-text: the newest entry that strictly
//! extends the text currently being typed.
use std::path::PathBuf;

/// Keep at most this many recent commands on disk.
const MAX: usize = 500;

fn path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("crew").join("far-history"))
}

/// Non-empty lines, oldest first (mirrors `crate::history::deserialize`).
fn deserialize(s: &str) -> Vec<String> {
    s.lines()
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect()
}

/// The Far command bar's persisted history. `cursor` tracks an in-progress
/// Up/Down browse: `None` means the bar shows live typed text; `Some(i)`
/// means it shows `entries[i]`. `stash` holds the text that was being typed
/// when browsing started, restored once Down passes the newest entry.
pub(crate) struct CmdHistory {
    entries: Vec<String>,
    cursor: Option<usize>,
    stash: String,
}

impl CmdHistory {
    /// Load the persisted history (empty if the file is missing/unreadable).
    pub(crate) fn load() -> Self {
        let entries = path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .map(|s| deserialize(&s))
            .unwrap_or_default();
        Self {
            entries,
            cursor: None,
            stash: String::new(),
        }
    }

    /// Build a history directly from `entries` (oldest first) — for tests
    /// that need known content without touching the filesystem.
    #[cfg(test)]
    pub(crate) fn from_entries(entries: Vec<String>) -> Self {
        Self {
            entries,
            cursor: None,
            stash: String::new(),
        }
    }

    /// Record a run command: skip blanks and immediate repeats, cap at
    /// `MAX` (oldest dropped), persist, and end any active browse.
    pub(crate) fn push(&mut self, cmd: &str) {
        self.cursor = None;
        self.stash.clear();
        if cmd.is_empty() || self.entries.last().map(String::as_str) == Some(cmd) {
            return;
        }
        self.entries.push(cmd.to_string());
        if self.entries.len() > MAX {
            let drop = self.entries.len() - MAX;
            self.entries.drain(..drop);
        }
        self.save();
    }

    fn save(&self) {
        let Some(p) = path() else { return };
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(p, self.entries.join("\n"));
    }

    /// Up: recall the previous (older) entry, stashing `current` the first
    /// time this is called since the last edit/push. `None` with no history.
    pub(crate) fn prev(&mut self, current: &str) -> Option<&str> {
        if self.entries.is_empty() {
            return None;
        }
        let i = match self.cursor {
            None => {
                self.stash = current.to_string();
                self.entries.len() - 1
            }
            Some(0) => 0,
            Some(i) => i - 1,
        };
        self.cursor = Some(i);
        Some(&self.entries[i])
    }

    /// Down: recall the next (newer) entry, or restore the stashed typed
    /// text once past the newest. `None` when not currently browsing.
    pub(crate) fn next(&mut self, _current: &str) -> Option<&str> {
        let i = self.cursor?;
        if i + 1 < self.entries.len() {
            self.cursor = Some(i + 1);
            Some(&self.entries[i + 1])
        } else {
            self.cursor = None;
            Some(self.stash.as_str())
        }
    }

    /// The newest entry that strictly extends `prefix` (`None` for an empty
    /// prefix — no ghost on an empty bar — or no match).
    pub(crate) fn ghost(&self, prefix: &str) -> Option<&str> {
        if prefix.is_empty() {
            return None;
        }
        self.entries
            .iter()
            .rev()
            .find(|e| e.starts_with(prefix) && e.len() > prefix.len())
            .map(String::as_str)
    }
}

/// Serialises tests that mutate `$HOME` to point at a tempdir — several
/// tests below load/save real history files and would race under the
/// default parallel test runner. Mirrors `crate::palette::test_guard`.
#[cfg(test)]
pub(crate) fn test_guard() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Point `$HOME` at a fresh tempdir for the duration of `f`, then
    /// restore it. Callers must hold `test_guard()` first.
    fn with_tmp_home<T>(f: impl FnOnce() -> T) -> T {
        let dir = tempfile::tempdir().unwrap();
        let prev = std::env::var_os("HOME");
        std::env::set_var("HOME", dir.path());
        let out = f();
        match prev {
            Some(p) => std::env::set_var("HOME", p),
            None => std::env::remove_var("HOME"),
        }
        out
    }

    #[test]
    fn load_is_empty_when_no_file_exists() {
        let _g = test_guard();
        with_tmp_home(|| {
            assert!(CmdHistory::load().entries.is_empty());
        });
    }

    #[test]
    fn push_persists_and_reloads() {
        let _g = test_guard();
        with_tmp_home(|| {
            let mut h = CmdHistory::load();
            h.push("ls");
            h.push("cargo test");
            let reloaded = CmdHistory::load();
            assert_eq!(
                reloaded.entries,
                vec!["ls".to_string(), "cargo test".to_string()]
            );
        });
    }

    #[test]
    fn push_skips_blank_and_adjacent_duplicate() {
        let _g = test_guard();
        with_tmp_home(|| {
            let mut h = CmdHistory::load();
            h.push("ls");
            h.push("ls"); // adjacent dupe, skipped
            h.push(""); // blank, skipped
            h.push("pwd");
            h.push("ls"); // not adjacent (pwd in between) — kept
            assert_eq!(
                h.entries,
                vec!["ls".to_string(), "pwd".to_string(), "ls".to_string()]
            );
        });
    }

    #[test]
    fn push_caps_at_max_dropping_oldest() {
        let _g = test_guard();
        with_tmp_home(|| {
            let mut h = CmdHistory::load();
            for i in 0..MAX + 10 {
                h.push(&format!("cmd{i}"));
            }
            assert_eq!(h.entries.len(), MAX);
            assert_eq!(h.entries.first().unwrap(), "cmd10"); // oldest 10 dropped
            assert_eq!(h.entries.last().unwrap(), &format!("cmd{}", MAX + 9));
        });
    }

    #[test]
    fn prev_next_cycle_and_restore_typed_text() {
        let mut h = CmdHistory::from_entries(vec!["ls".into(), "pwd".into(), "cargo test".into()]);
        assert_eq!(h.prev("half-typed"), Some("cargo test")); // newest first
        assert_eq!(h.prev("half-typed"), Some("pwd"));
        assert_eq!(h.prev("half-typed"), Some("ls")); // oldest
        assert_eq!(h.prev("half-typed"), Some("ls")); // stays at oldest
        assert_eq!(h.next("ls"), Some("pwd"));
        assert_eq!(h.next("pwd"), Some("cargo test"));
        assert_eq!(h.next("cargo test"), Some("half-typed")); // restored
        assert_eq!(h.next("anything"), None); // not browsing anymore
    }

    #[test]
    fn prev_with_no_history_returns_none() {
        let mut h = CmdHistory::from_entries(vec![]);
        assert_eq!(h.prev("typed"), None);
    }

    #[test]
    fn ghost_matches_the_newest_extending_entry() {
        let h = CmdHistory::from_entries(vec![
            "cargo build".into(),
            "cargo check".into(),
            "cargo test".into(),
        ]);
        assert_eq!(h.ghost("cargo"), Some("cargo test")); // newest wins
        assert_eq!(h.ghost("cargo test"), None); // no STRICT extension
        assert_eq!(h.ghost("zz"), None); // no match
    }

    #[test]
    fn ghost_is_none_on_an_empty_bar() {
        let h = CmdHistory::from_entries(vec!["cargo test".into()]);
        assert_eq!(h.ghost(""), None);
    }
}

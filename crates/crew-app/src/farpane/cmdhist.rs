//! Command history for the Far command bar: persisted beside the existing
//! chat-input history (same `dirs` base as `crate::history`, a sibling file
//! named `far-history`), newline-delimited, deduped against the immediately
//! preceding entry, capped at 500 entries (oldest dropped), loaded once per
//! pane. Also serves fish-style ghost-text: the newest entry that strictly
//! extends the text currently being typed.
use std::path::PathBuf;

/// Keep at most this many recent commands on disk.
const MAX: usize = 500;

/// `<config_dir>/crew/far-history` — unless `CREW_FAR_HISTORY_PATH` is set
/// (non-empty), in which case that path wins.
///
/// The override exists for test isolation, mirroring
/// `crew_plugin::credentials::path`. Pointing `$HOME` at a tempdir is enough
/// on Unix, where `dirs::config_dir()` derives from it, but on Windows that
/// function reads the Known Folder API and ignores the environment outright:
/// the "isolated" history tests were reading and writing the real user
/// profile, so they came back holding 500 entries that
/// `push_caps_at_max_dropping_oldest` had pushed. Do not remove this to
/// "simplify" the function; it is the only seam that isolates this store on
/// Windows.
fn path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("CREW_FAR_HISTORY_PATH") {
        if !p.is_empty() {
            return Some(PathBuf::from(p));
        }
    }
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

    // Like `crate::history`, this file is last-writer-wins across panes and
    // instances (no lock, no merge) — a concurrent save can clobber another
    // process's, and `deserialize` tolerates a torn tail from a save that
    // raced a read.
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

#[cfg(test)]
#[path = "cmdhist_tests.rs"]
mod tests;

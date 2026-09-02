//! The watchlist: what crew is waiting to do, in a file that is only ever appended to.
//!
//! Deliberately the ledger's discipline (`crew-plugin/src/broker/ledger.rs`) rather than a config
//! file that gets rewritten: a thing that acts while nobody is awake has to be auditable for
//! exactly the reason it is useful, and the interesting record is always the one written just
//! before something went wrong. So a cancellation is an APPENDED tombstone, a firing is an
//! APPENDED fact, and the live watchlist is what you get by folding the log — never a rewrite,
//! never a truncation.
//!
//! It sits beside the ledger for the same reason the ledger sits beside the config: a cache or
//! temp directory is somewhere a cleaner may empty, and an alarm that silently stopped existing
//! is worse than one that never existed.
use std::io::Write;
use std::path::PathBuf;

use super::intent::{Intent, Repeat};

/// One line of the log. Tagged, so a reader that meets a future entry kind can skip it instead
/// of failing the whole fold.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "op")]
pub(crate) enum Entry {
    /// A new intent joined the watchlist.
    Added { intent: Intent },
    /// One fired. `next_ms` is where a repeat landed, `None` when that was the last of it.
    Fired {
        id: String,
        at_ms: u64,
        next_ms: Option<u64>,
        /// Occurrences skipped because the machine was not running for them.
        #[serde(default)]
        skipped: u64,
    },
    /// Somebody called it off.
    Cancelled { id: String, at_ms: u64 },
    /// The next firing was pushed to `to_ms`; a repeat's cadence stays where it was.
    Snoozed { id: String, to_ms: u64, at_ms: u64 },
}

/// The watchlist file's home, beside the ledger and the config.
pub(crate) fn default_path() -> PathBuf {
    dirs::config_dir()
        .map(|d| d.join("crew"))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("watchlist.jsonl")
}

/// An append-only watchlist at a path.
pub(crate) struct Watchlist {
    path: PathBuf,
}

impl Watchlist {
    pub(crate) fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    #[cfg(test)]
    pub(crate) fn path(&self) -> &std::path::Path {
        &self.path
    }

    /// Append one entry, flushed to the OS before returning.
    pub(crate) fn append(&self, e: &Entry) -> std::io::Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut line = serde_json::to_string(e).map_err(std::io::Error::other)?;
        line.push('\n');
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        f.write_all(line.as_bytes())?;
        f.flush()
    }

    /// Every entry in the order it was written, plus the number of unreadable lines stepped
    /// over. A crash mid-append must cost one line, not the whole watchlist.
    pub(crate) fn entries(&self) -> (Vec<Entry>, usize) {
        let Ok(text) = std::fs::read_to_string(&self.path) else {
            return (Vec::new(), 0);
        };
        let mut out = Vec::new();
        let mut bad = 0;
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            match serde_json::from_str::<Entry>(line) {
                Ok(e) => out.push(e),
                Err(_) => bad += 1,
            }
        }
        (out, bad)
    }

    /// What is still standing, soonest first: added, minus cancelled, minus one-shots that have
    /// fired, with every repeat carrying the fire time its last firing rolled it to.
    pub(crate) fn live(&self) -> Vec<Intent> {
        let mut live: Vec<Intent> = Vec::new();
        for e in self.entries().0 {
            match e {
                Entry::Added { intent } => {
                    // An id that somehow appears twice keeps the LATER record: replaying an
                    // append that was already applied must not leave two of the same alarm.
                    live.retain(|i| i.id != intent.id);
                    live.push(intent);
                }
                Entry::Fired { id, next_ms, .. } => match next_ms {
                    Some(next) => {
                        if let Some(i) = live.iter_mut().find(|i| i.id == id) {
                            i.fire_ms = next;
                            i.anchor_ms = None;
                        }
                    }
                    None => live.retain(|i| i.id != id),
                },
                Entry::Cancelled { id, .. } => live.retain(|i| i.id != id),
                Entry::Snoozed { id, to_ms, .. } => {
                    if let Some(i) = live.iter_mut().find(|i| i.id == id) {
                        i.anchor_ms = Some(i.anchor_ms.unwrap_or(i.fire_ms));
                        i.fire_ms = to_ms;
                    }
                }
            }
        }
        live.sort_by_key(|i| (i.fire_ms, i.id.clone()));
        live
    }

    /// The id a new intent gets: one past the highest ever ADDED, cancelled and fired ones
    /// included. Reusing `w3` for something else would make an old log line read as a lie.
    pub(crate) fn next_id(&self) -> String {
        let highest = self
            .entries()
            .0
            .iter()
            .filter_map(|e| match e {
                Entry::Added { intent } => intent.id.strip_prefix('w')?.parse::<u64>().ok(),
                _ => None,
            })
            .max()
            .unwrap_or(0);
        format!("w{}", highest + 1)
    }

    /// Register one. Returns the intent as stored, id and all.
    pub(crate) fn add(
        &self,
        text: &str,
        to: &str,
        fire_ms: u64,
        repeat: Repeat,
        now_ms: u64,
    ) -> std::io::Result<Intent> {
        let intent = Intent {
            id: self.next_id(),
            text: text.to_string(),
            to: to.to_string(),
            fire_ms,
            repeat,
            created_ms: now_ms,
            anchor_ms: None,
        };
        self.append(&Entry::Added {
            intent: intent.clone(),
        })?;
        Ok(intent)
    }

    /// Record that `intent` fired at `now_ms`, rolling a repeat forward past now. Returns how
    /// many occurrences the roll stepped over, which the firing says out loud.
    pub(crate) fn record_fire(&self, intent: &Intent, now_ms: u64) -> std::io::Result<u64> {
        let rolled = intent.advance(now_ms);
        self.append(&Entry::Fired {
            id: intent.id.clone(),
            at_ms: now_ms,
            next_ms: rolled.map(|r| r.next),
            skipped: rolled.map(|r| r.skipped).unwrap_or(0),
        })?;
        Ok(rolled.map(|r| r.skipped).unwrap_or(0))
    }

    /// Call one off. `false` when nothing by that id is standing — a cancel that matched
    /// nothing must say so rather than reporting a success it did not have.
    pub(crate) fn cancel(&self, id: &str, now_ms: u64) -> std::io::Result<bool> {
        if !self.live().iter().any(|i| i.id == id) {
            return Ok(false);
        }
        self.append(&Entry::Cancelled {
            id: id.to_string(),
            at_ms: now_ms,
        })?;
        Ok(true)
    }
}

#[cfg(test)]
#[path = "intentlog_tests.rs"]
mod tests;

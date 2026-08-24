//! The ledger: what crew actually did, in a file that is only ever appended to.
//!
//! An assistant trusted with mail, money and someone's front door has to be auditable after the
//! fact, and "after the fact" is the hard part — the interesting record is always the one written
//! before something went wrong. So this is deliberately dull: one JSON object per line, opened
//! with O_APPEND, flushed to the OS on every write, never rewritten, never truncated.
//!
//! It is NOT `activity.log`. That file is truncated on every process start and skipped under
//! test, which is right for a session log and disqualifying for an audit trail.
use std::io::Write;
use std::path::{Path, PathBuf};

use super::approval::{Outcome, Requester};
use super::tier::Tier;

/// One thing that happened, in the order it happened.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Record {
    /// Milliseconds since the Unix epoch.
    pub ts_ms: u64,
    /// `server:name` of the tool.
    pub tool: String,
    /// Reversibility class at the time of the call — stored as a word, not a number, so an
    /// old ledger stays readable if the enum ever changes shape.
    pub tier: String,
    /// Who asked: `pane`, `channel:<addr>`, or `trigger:<name>`.
    pub requester: String,
    /// What the gate decided: `allow`, `ask`, or `deny`.
    pub decision: String,
    /// How it ended, once known: `granted`, `denied`, `timed_out`, `ran`, `failed`.
    pub outcome: String,
    /// Free-text detail — a denial reason, an error, the approval id.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
}

/// Render a requester the way the ledger stores it.
pub fn requester_label(r: &Requester) -> String {
    match r {
        Requester::LocalPane => "pane".to_string(),
        Requester::Channel(c) => format!("channel:{c}"),
        Requester::Trigger(t) => format!("trigger:{t}"),
    }
}

/// Render an outcome the way the ledger stores it.
pub fn outcome_label(o: Outcome) -> &'static str {
    match o {
        Outcome::Granted => "granted",
        Outcome::Denied => "denied",
        Outcome::TimedOut => "timed_out",
    }
}

impl Record {
    /// A record for a decision the gate just made.
    pub fn decided(
        tool: &str,
        tier: Tier,
        requester: &Requester,
        decision: &str,
        note: &str,
    ) -> Self {
        Self {
            ts_ms: now_ms(),
            tool: tool.to_string(),
            tier: tier.label().to_string(),
            requester: requester_label(requester),
            decision: decision.to_string(),
            outcome: String::new(),
            note: note.to_string(),
        }
    }

    /// Fill in how it ended.
    pub fn with_outcome(mut self, outcome: &str) -> Self {
        self.outcome = outcome.to_string();
        self
    }
}

/// Milliseconds since the Unix epoch, saturating at 0 before it.
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// The ledger file's default home: beside the config, not in a temp or cache directory that a
/// cleaner may empty.
pub fn default_path() -> PathBuf {
    dirs::config_dir()
        .map(|d| d.join("crew"))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ledger.jsonl")
}

/// An append-only ledger at a path.
pub struct Ledger {
    path: PathBuf,
}

impl Ledger {
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Append one record. Opened `append(true)` every time rather than holding a handle: two
    /// processes writing the same ledger (the daemon and a broker child) then interleave whole
    /// lines instead of overwriting each other at a stale offset.
    pub fn append(&self, r: &Record) -> std::io::Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut line = serde_json::to_string(r).map_err(std::io::Error::other)?;
        line.push('\n');
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        f.write_all(line.as_bytes())?;
        // The record that matters is the one written just before a crash, so the write reaches
        // the OS before this returns rather than sitting in a buffer that dies with us.
        f.flush()
    }

    /// Read the whole ledger, oldest first, plus the number of unreadable lines skipped.
    ///
    /// A truncated final line — a crash mid-append — must not make the entire history
    /// unreadable, so parse failures are counted and stepped over rather than propagated.
    pub fn read(&self) -> (Vec<Record>, usize) {
        let Ok(text) = std::fs::read_to_string(&self.path) else {
            return (Vec::new(), 0);
        };
        let mut out = Vec::new();
        let mut bad = 0;
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            match serde_json::from_str::<Record>(line) {
                Ok(r) => out.push(r),
                Err(_) => bad += 1,
            }
        }
        (out, bad)
    }
}

#[cfg(test)]
#[path = "ledger_tests.rs"]
mod tests;

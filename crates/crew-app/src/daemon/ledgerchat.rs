//! "what have you done?" — the ledger, read from a channel.
//!
//! The irreversible approvals arrive on your phone (`answers.rs`), so from
//! your pocket you can say yes and could not see what you had already said
//! yes to: the ledger was readable from `/tools` and `crew ledger`, both of
//! which need the machine in front of you. This is the third face, and like
//! the clock's it claims only what it names — "tools", "ledger", "what have
//! you done" — so everything else stays a task for an agent.
use crew_plugin::ledger::Record;

use crate::toolsrow::{ago, detail, mark};

/// Rows a phone answer carries. A screen, not a listing: the last few, and
/// a word to narrow them.
pub(crate) const ROWS: usize = 10;

/// The filter word, if `said` asks for the ledger. `Some("")` is the plain
/// ask; `Some("gmail")` is "tools gmail".
pub(crate) fn read(said: &str) -> Option<String> {
    let lower = said.trim().to_lowercase();
    let mut words = lower.split_whitespace();
    let first = words.next()?;
    if matches!(first, "tools" | "/tools" | "ledger" | "/ledger") {
        return Some(words.collect::<Vec<_>>().join(" "));
    }
    // "what have you done", "what did you do", "what have you run" — and a
    // trailing word narrows: "what did you do with gmail".
    let asked = [
        "what have you done",
        "what did you do",
        "what have you run",
        "what have you been doing",
    ]
    .iter()
    .find_map(|q| lower.strip_prefix(q))?;
    let rest = asked.trim_start_matches([',', '?', ' ']);
    let rest = rest.strip_prefix("with ").unwrap_or(rest);
    Some(rest.trim_end_matches('?').trim().to_string())
}

/// The last [`ROWS`] records matching `filter`, oldest first so the newest
/// is the line nearest the reader's thumb.
pub(crate) fn answer(records: &[Record], filter: &str, now_ms: u64) -> String {
    let filter = filter.to_lowercase();
    let hits: Vec<&Record> = records
        .iter()
        .filter(|r| {
            filter.is_empty()
                || r.tool.to_lowercase().contains(&filter)
                || r.requester.to_lowercase().contains(&filter)
                || r.outcome.to_lowercase().contains(&filter)
        })
        .collect();
    if hits.is_empty() {
        return if filter.is_empty() {
            "I have not run anything yet".to_string()
        } else {
            format!("nothing in the ledger matches {filter:?}")
        };
    }
    let shown = &hits[hits.len().saturating_sub(ROWS)..];
    let mut lines: Vec<String> = shown
        .iter()
        .map(|r| {
            let mut l = format!("{} {} {}", ago(r.ts_ms, now_ms), mark(r), r.tool);
            if let Some(d) = detail(r) {
                l.push_str(" \u{2014} ");
                l.push_str(&d);
            }
            l
        })
        .collect();
    if hits.len() > ROWS {
        lines.insert(0, format!("{} more before these", hits.len() - ROWS));
    }
    lines.join("\n")
}

impl super::Daemon {
    /// Answer a ledger question from a channel, or `None` when the message is not one.
    ///
    /// The same exception the clock takes: a conversation blocked on an approval is
    /// answering a question, and nothing it says is read as a command.
    pub(crate) fn ledger_chat(&self, from: &str, said: &str, now_ms: u64) -> Option<String> {
        if self.bridge.is_awaiting(from) {
            return None;
        }
        let filter = read(said)?;
        let (records, _) = crew_plugin::ledger::Ledger::at(&self.ledger).read();
        Some(answer(&records, &filter, now_ms))
    }
}

#[cfg(test)]
#[path = "ledgerchat_tests.rs"]
mod tests;

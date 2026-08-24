//! `crew ledger` — read back what crew did. A trail nobody can read is not an audit trail, so
//! the reader ships with the writer.
use crew_plugin::ledger::{Ledger, Record};

/// Render one record as a line: time, decision, tier, requester, tool, note.
pub(crate) fn line(r: &Record) -> String {
    let when = stamp(r.ts_ms);
    let outcome = if r.outcome.is_empty() {
        String::new()
    } else {
        format!(" \u{2192} {}", r.outcome)
    };
    let note = if r.note.is_empty() {
        String::new()
    } else {
        format!("  ({})", r.note)
    };
    format!(
        "{when}  {:<5} {:<12} {:<22} {}{outcome}{note}",
        r.decision, r.tier, r.requester, r.tool
    )
}

/// `HH:MM:SS` in UTC from an epoch-millisecond stamp. Deliberately arithmetic rather than a date
/// library: the ledger's job is ordering and correlation, not calendars.
pub(crate) fn stamp(ts_ms: u64) -> String {
    let secs = ts_ms / 1000;
    let (h, m, s) = ((secs / 3600) % 24, (secs / 60) % 60, secs % 60);
    format!("{h:02}:{m:02}:{s:02}")
}

/// `crew ledger [--limit N]`. `None` when this is not a ledger invocation.
pub(crate) fn dispatch_cli() -> Option<i32> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) != Some("ledger") {
        return None;
    }
    let limit = args
        .iter()
        .position(|a| a == "--limit")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(50);
    let l = Ledger::at(crew_plugin::ledger::default_path());
    let (records, bad) = l.read();
    if records.is_empty() {
        println!("the ledger is empty ({})", l.path().display());
        return Some(0);
    }
    for r in records.iter().rev().take(limit).rev() {
        println!("{}", line(r));
    }
    if bad > 0 {
        // Never hidden: an unreadable line is exactly the kind of thing an audit needs told.
        eprintln!("{bad} unreadable line(s) skipped");
    }
    Some(0)
}

#[cfg(test)]
#[path = "ledgercli_tests.rs"]
mod tests;

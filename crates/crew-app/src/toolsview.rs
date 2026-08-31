//! `/tools`: what the agents did to your machine.
//!
//! Every tool call an agent makes passes one gate and appends one line to
//! `~/.config/crew/ledger.jsonl` — its tier, who asked, what the gate decided
//! and how it ended. That has been true since the gate landed, and NOTHING HAS
//! EVER READ IT: `Ledger::read` had no caller outside its own tests. An audit
//! trail nobody can open is not an audit trail.
//!
//! Rendered to a temp file and opened in the file viewer, the way `/blocks`
//! and `/out` are — a history is something you scroll and search, not
//! something to paste into the conversation it is a history of.
use crew_plugin::ledger::{Ledger, Record};

#[cfg(test)]
#[path = "toolsview_tests.rs"]
mod tests;

/// Rows the listing shows. The ledger is append-only and unbounded — it is the
/// record of a machine, not of a session — so the view takes the tail. A
/// thousand rows is more than anyone scrolls and small enough to render.
pub(crate) const MAX_ROWS: usize = 1_000;

/// Columns the tier field takes, so the tools line up under it.
const TIER_W: usize = 12;

/// How a call ended, in one glyph plus its word.
///
/// A record with no outcome yet is `·`, not `✓`: the ledger notes the DECISION
/// when the gate makes it and the OUTCOME when the call returns, and a crash
/// between the two leaves a real row with nothing after it. Drawing that as
/// success would be inventing an answer.
fn outcome(r: &Record) -> String {
    match (r.outcome.as_str(), r.decision.as_str()) {
        ("", "deny") => "\u{2717} denied".into(),
        ("", _) => "\u{b7}".into(),
        ("ran" | "granted", _) => "\u{2713} ran".into(),
        (o, _) => format!("\u{2717} {o}"),
    }
}

/// `14:03:22` in local time, or `--:--:--` for a stamp we cannot place.
fn clock(ts_ms: u64) -> String {
    let secs = ts_ms / 1000;
    // Wall-clock seconds into the day. Deliberately NOT a calendar conversion:
    // crew has no date library in its dependencies, and the question this
    // listing answers ("what ran, in what order, how recently") needs the time
    // of day, not the date.
    let day = secs % 86_400;
    format!("{:02}:{:02}:{:02}", day / 3600, (day % 3600) / 60, day % 60)
}

/// The listing for `records`, newest first, as viewer text.
///
/// `bad` is the count of unreadable lines the ledger stepped over; it is
/// REPORTED rather than swallowed, because a history with a hole in it that
/// says so is worth more than one that quietly shows fewer rows.
pub(crate) fn listing(records: &[Record], bad: usize) -> String {
    let mut out = String::from("# tools \u{b7} what agents ran\n\n");
    if records.is_empty() {
        out.push_str("Nothing yet. Every tool an agent calls lands here.\n");
        return out;
    }
    let shown = records.len().min(MAX_ROWS);
    for r in records.iter().rev().take(shown) {
        out.push_str(&format!(
            "{}  {:<TIER_W$}  {:<12}  {:<24}  {}\n",
            clock(r.ts_ms),
            r.tier,
            outcome(r),
            r.tool,
            r.requester,
        ));
        if !r.note.is_empty() {
            out.push_str(&format!("{:>10}  {}\n", "", r.note));
        }
    }
    if records.len() > shown {
        out.push_str(&format!(
            "\n\u{2026} {} older call(s) not shown\n",
            records.len() - shown
        ));
    }
    if bad > 0 {
        out.push_str(&format!("\n\u{26a0} {bad} unreadable line(s) skipped\n"));
    }
    out
}

impl crate::app::CrewApp {
    /// `/tools` — open the action ledger in the viewer.
    pub(crate) fn open_tools(&mut self) {
        let (records, bad) = Ledger::at(crew_plugin::ledger::default_path()).read();
        let text = listing(&records, bad);
        // Keyed to no pane: the ledger is one machine-wide file, so two panes
        // opening it are looking at the same thing and may share the file.
        let path = crate::lastout::temp_path(usize::MAX, "tools");
        if let Err(e) = std::fs::write(&path, text) {
            self.set_status(format!("tools: cannot write: {e}"));
            return;
        }
        let before = self.panes.len();
        self.open_view(&path.to_string_lossy());
        self.name_last_view("tools");
        self.mark_last_view_ephemeral(before);
    }
}

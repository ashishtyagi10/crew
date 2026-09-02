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
use crate::toolsrow::{ago, detail, fit, mark, wrap, AGO_W, DETAIL_INDENT, ROW_W, TOOL_W};
use crew_plugin::ledger::{Ledger, Record};

#[cfg(test)]
#[path = "toolsview_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "toolsfit_tests.rs"]
mod fit_tests;

/// Rows the listing shows. The ledger is append-only and unbounded — it is the
/// record of a machine, not of a session — so the view takes the tail. A
/// thousand rows is more than anyone scrolls and small enough to render.
pub(crate) const MAX_ROWS: usize = 1_000;

/// Whether `r` matches `filter` — a case-insensitive substring of any field a
/// person would search by.
///
/// One term against every field rather than a query language: the questions
/// this listing gets asked are "what did `sys:run` do", "what was denied" and
/// "what did the phone ask for", and each of those is one word that happens to
/// live in a different column. Making the user learn which column would be
/// making them do the search's job.
fn matches(r: &Record, filter: &str) -> bool {
    if filter.is_empty() {
        return true;
    }
    let f = filter.to_lowercase();
    [
        &r.tool,
        &r.tier,
        &r.requester,
        &r.decision,
        &r.outcome,
        &r.note,
    ]
    .iter()
    .any(|field| field.to_lowercase().contains(&f))
}

/// The listing for `records`, newest first, as viewer text.
///
/// `bad` is the count of unreadable lines the ledger stepped over; it is
/// REPORTED rather than swallowed, because a history with a hole in it that
/// says so is worth more than one that quietly shows fewer rows.
pub(crate) fn listing(records: &[Record], bad: usize, filter: &str, now_ms: u64) -> String {
    let mut out = match filter.is_empty() {
        true => String::from("# tools \u{b7} what agents ran\n"),
        false => format!("# tools \u{b7} matching \u{201c}{filter}\u{201d}\n"),
    };
    let hits: Vec<&Record> = records.iter().filter(|r| matches(r, filter)).collect();
    if hits.is_empty() {
        // A filter that matched nothing must not read like an empty ledger:
        // "there is no history" and "your search found none of it" are
        // different answers and only one of them means you typed it wrong.
        out.push_str(&match (records.is_empty(), filter.is_empty()) {
            (true, _) => "Nothing yet. Every tool an agent calls lands here.\n".to_string(),
            // Two lines, a sentence each: on one line this was the widest
            // row the listing could produce and wrapped in a 75-column pane.
            (false, _) => format!(
                "No call matches \u{201c}{filter}\u{201d}.\n{} call(s) recorded \u{2014} \
                 /tools with no term lists them.\n",
                records.len()
            ),
        });
        return out;
    }
    // A SNAPSHOT, and it says so. The times below are relative, which is what
    // makes them right in every timezone — and also what makes a listing left
    // open on screen quietly wrong, since "30s ago" stays "30s ago" while the
    // agents keep working. The count is the other half: a filtered view of
    // three rows out of nine hundred should not look like the whole history.
    out.push_str(&format!(
        "{} call(s) \u{b7} times as of opening\n/tools re-reads the ledger\n",
        hits.len()
    ));
    // What kind of history this is, before a line of it: the tiers in view,
    // and how many did not simply run.
    for row in crate::toolstally::tally(&hits) {
        out.push_str(&row);
        out.push('\n');
    }
    out.push('\n');
    let shown = hits.len().min(MAX_ROWS);
    for r in hits.iter().rev().take(shown) {
        out.push_str(&format!(
            "{:<AGO_W$}  {}  {:<TOOL_W$}  {}\n",
            ago(r.ts_ms, now_ms),
            mark(r),
            fit(&r.tool, TOOL_W),
            r.tier,
        ));
        if let Some(d) = detail(r) {
            for line in wrap(&d, ROW_W - DETAIL_INDENT) {
                out.push_str(&format!("{:DETAIL_INDENT$}{line}\n", ""));
            }
        }
    }
    if hits.len() > shown {
        out.push_str(&format!(
            "\n\u{2026} {} older call(s) not shown\n",
            hits.len() - shown
        ));
    }
    if bad > 0 {
        out.push_str(&format!("\n\u{26a0} {bad} unreadable line(s) skipped\n"));
    }
    out
}

impl crate::app::CrewApp {
    /// `/tools [term]` — open the action ledger in the viewer, optionally
    /// narrowed to the calls matching `term`.
    pub(crate) fn open_tools(&mut self, filter: &str) {
        let (records, bad) = Ledger::at(crew_plugin::ledger::default_path()).read();
        let text = listing(&records, bad, filter, crate::chattime::unix_now_ms());
        // Keyed to no pane: the ledger is one machine-wide file, so two panes
        // opening it are looking at the same thing and may share the file.
        let path = crate::lastout::temp_path(usize::MAX, "tools");
        if let Err(e) = std::fs::write(&path, text) {
            self.set_status(format!("tools: cannot write: {e}"));
            return;
        }
        let before = self.panes.len();
        self.open_view(&path.to_string_lossy());
        self.name_last_view(&match filter.is_empty() {
            true => "tools".to_string(),
            false => format!("tools \u{b7} {filter}"),
        });
        self.mark_last_view_ephemeral(before);
    }
}

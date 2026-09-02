//! One row of the `/tools` listing: how a call reads, and how it fits.
//!
//! Pure formatting, split from [`crate::toolsview`] so the command and the
//! ledger read stay separate from the question of what a row looks like — and
//! so both stay inside the line cap.
use crew_plugin::ledger::Record;

#[cfg(test)]
#[path = "toolsrow_tests.rs"]
mod tests;

/// Columns the tool name takes, so the tiers line up under each other.
///
/// The whole row is built to fit a NARROW pane. It was 80 columns wide — a
/// clock, a padded tier, a padded outcome, a padded tool and a requester — and
/// a viewer opened as one tile of a 2×2 grid is nearer 50, so every single row
/// wrapped and the listing was unreadable in the place it is most often
/// opened.
pub(crate) const TOOL_W: usize = 20;

/// Columns the relative time takes (`42s ago`, `2d ago`).
pub(crate) const AGO_W: usize = 8;

/// The indent a detail line sits at, under the row it belongs to.
pub(crate) const DETAIL_INDENT: usize = 13;

/// Total columns a row — or a wrapped detail line — may take.
///
/// The main row is exactly this wide with its widest tier (`irreversible`),
/// and it is what the narrowest tile the listing is opened in can show:
/// `toolshot_tests` drew it at sixty and watched every irreversible row
/// break `irreve↪rsible` and every note wrap mid-word a second time in the
/// viewer.
pub(crate) const ROW_W: usize = 47;

/// How a call ended, in ONE glyph.
///
/// The word is gone from the main row: `ran` beside a tick says nothing the
/// tick did not, and it cost eleven columns on every row to repeat it. What is
/// unusual — a denial, a failure, a timeout — moves to the detail line, where
/// it appears beside the reason it happened.
///
/// A record with no outcome yet is `·`, not `✓`: the ledger notes the DECISION
/// when the gate makes it and the OUTCOME when the call returns, and a crash
/// between the two leaves a real row with nothing after it. Drawing that as
/// success would be inventing an answer.
pub(crate) fn mark(r: &Record) -> char {
    match (r.outcome.as_str(), r.decision.as_str()) {
        ("", "deny") => '\u{2717}',
        ("", _) => '\u{b7}',
        ("ran" | "granted", _) => '\u{2713}',
        _ => '\u{2717}',
    }
}

/// `server:tool` in at most `max` columns, cut in the MIDDLE.
///
/// Real tool names run long — `google_workspace:search_gmail_messages` is 38
/// columns — and a head clip loses the tool while a tail clip loses the
/// server. Both ends are what tells two calls apart, so the cut goes where
/// the least identity lives.
pub(crate) fn fit(name: &str, max: usize) -> String {
    let n = name.chars().count();
    if n <= max {
        return name.to_string();
    }
    let keep = max.saturating_sub(1);
    let head = keep.div_ceil(2);
    let tail = keep - head;
    let chars: Vec<char> = name.chars().collect();
    let mut out: String = chars[..head].iter().collect();
    out.push('\u{2026}');
    out.extend(&chars[n - tail..]);
    out
}

/// Word-wrap `text` to `width`, hard-breaking only a word that cannot fit on
/// a line of its own.
///
/// A note is free text of unknown length — an error, a denial reason, whatever
/// a tool said on its way out — and the detail line is the one place it is
/// ever shown. Clipping it would drop the tail of exactly the message the line
/// exists to carry, so it wraps instead.
pub(crate) fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut cur = String::new();
    for word in text.split_whitespace() {
        let w = word.chars().count();
        let cur_w = cur.chars().count();
        if !cur.is_empty() && cur_w + 1 + w > width {
            lines.push(std::mem::take(&mut cur));
        }
        if w > width {
            // Longer than a whole line: break it rather than overflow.
            let mut rest = word;
            while rest.chars().count() > width {
                let head: String = rest.chars().take(width).collect();
                lines.push(head);
                rest = &rest[rest
                    .char_indices()
                    .nth(width)
                    .map_or(rest.len(), |(i, _)| i)..];
            }
            cur = rest.to_string();
            continue;
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(word);
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

/// The line under a row, or `None` when there is nothing unusual to say.
///
/// The common call is a person at this keyboard running something that worked,
/// and a listing that spelled `ran` and `pane` on every one of those rows was
/// spending its width saying "normal". This says only what a reader would not
/// already assume: how it ended when that was not plainly "it ran", who asked
/// when it was not the person reading, and the note when there is one.
pub(crate) fn detail(r: &Record) -> Option<String> {
    let mut parts: Vec<&str> = Vec::new();
    if !matches!(r.outcome.as_str(), "ran" | "granted" | "") {
        parts.push(&r.outcome);
    }
    if r.outcome.is_empty() && r.decision == "deny" {
        parts.push("denied");
    }
    if r.requester != "pane" && !r.requester.is_empty() {
        parts.push(&r.requester);
    }
    if !r.note.is_empty() {
        parts.push(&r.note);
    }
    (!parts.is_empty()).then(|| parts.join(" \u{b7} "))
}

/// How long ago a call ran — `now`, `42s ago`, `3h ago`, `2d ago`.
///
/// It was a WALL CLOCK, and the wall clock was wrong: the time was computed as
/// seconds into the epoch day, which is UTC, under a doc comment claiming it
/// was local. Every row was off by the reader's offset from Greenwich, and the
/// comment said otherwise.
///
/// Fixing that properly wants a timezone database, and crew has no date
/// library among its dependencies — deliberately: every other time it shows
/// you (`chattime::rel_time`, the message cards, `/blocks`) is relative, for
/// the same reason. "What ran recently, and in what order" is the question
/// this listing is opened with, and a relative time answers it without owning
/// a timezone at all.
pub(crate) fn ago(ts_ms: u64, now_ms: u64) -> String {
    crate::chattime::rel_time(&ts_ms.to_string(), now_ms).unwrap_or_default()
}

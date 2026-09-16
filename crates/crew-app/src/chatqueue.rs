//! Messages typed while the crew is busy: queued instead of sent immediately
//! (see the queued-messages design doc), then flushed one at a time as each
//! turn settles (the flush itself lives in `chat::ChatPane::poll`, since it
//! needs private field access). This module holds the pure bits: the `/stop`
//! bypass check and the one-line "N queued" indicator that claims a row
//! above the composer, mirroring how `chatswarmview::swarm_rows` claims rows
//! for the live swarm block.
use crate::chat::ChatPane;

/// Whether `text` is (or starts) a `/stop` command — the one send that must
/// bypass the queue and reach the broker immediately even while busy, since
/// it's the cancel path for the in-flight run.
pub(crate) fn is_stop(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed == "/stop" || trimmed.starts_with("/stop ")
}

/// Whether `text` cancels EVERYTHING, i.e. a bare `/stop` with no task id.
///
/// The distinction matters because cancelling is what makes the pane idle,
/// and idle is what flushes the queue: a cancel-everything must take the
/// queue with it or it restarts the moment it lands. `/stop #2` is the other
/// thing — one of several parallel tasks called off, with the rest of the
/// session, and the rest of the queue, still meant.
pub(crate) fn is_stop_all(text: &str) -> bool {
    text.trim() == "/stop"
}

/// Queued messages listed under the summary before the rest become a count.
/// Three is what fits above a composer without the waiting outgrowing the
/// conversation it is waiting for.
pub(crate) const SHOWN: usize = 3;

/// Rows the queued-indicator claims: 0 when empty, else the summary line plus
/// one per message waiting (to [`SHOWN`], then a line for the remainder).
///
/// The summary comes FIRST so that a pane too short for the list degrades to
/// exactly what this surface drew before it had one: `chatplace::grants`
/// clamps every surface to the rows left, and the row it can always afford is
/// the one that says how many.
pub(crate) fn queued_rows(pane: &ChatPane) -> u16 {
    let n = pane.queued.len();
    if n == 0 {
        return 0;
    }
    let listed = n.min(SHOWN) + usize::from(n > SHOWN);
    1 + listed as u16
}

/// The waiting messages, in the order they will be sent, as the lines that go
/// under the summary.
///
/// WHY they are worth the rows: the queue could say how MANY were waiting and
/// never what, so a message typed five minutes ago behind a long run was
/// invisible until it sent itself. You could not tell whether the thing you
/// meant to ask was in there, and the only way to take one back was Esc,
/// which cancels the run as well and drops all of them.
pub(crate) fn listed(pane: &ChatPane) -> Vec<String> {
    let n = pane.queued.len();
    let mut out: Vec<String> = pane
        .queued
        .iter()
        .take(SHOWN)
        .enumerate()
        // Numbered, because the order is the whole point: this is what the
        // crew will be asked next, and next after that.
        .map(|(i, t)| format!("{}. {}", i + 1, flatten(t)))
        .collect();
    if n > SHOWN {
        out.push(format!("\u{2026} +{} more", n - SHOWN));
    }
    out
}

/// A queued message as one line. A pasted block is still one thing waiting,
/// and it claims one row like everything else here.
fn flatten(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
#[path = "chatqueue_tests.rs"]
mod tests;

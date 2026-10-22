//! Unread mail on top ("unread should always be on top", 2026-10-22): the
//! list is its unread threads, newest first, then the read ones.
//!
//! The order is made as a list comes, not as a row changes, and a thread
//! that was on top stays there until the list is asked for again by hand
//! (`R`, a search). Reading down the unread ones, the thread just read does
//! not drop out from under the selection, and neither does the check after
//! it, which lists the first page again because the read moved the mailbox.
//! Gmail's own "Unread first" keeps a read thread in its place the same way.
//! The threads on top sit under one band, `unread`, not the days' (`layout`).
use super::wire::Thread;

/// The band over the threads on top.
pub(crate) const BAND: &str = "unread";

/// Unread threads first, each part in the order it came; a thread of `was`,
/// the ones on top before this list came, stays on top.
pub(crate) fn order(rows: &mut [Thread], was: &[String]) {
    for t in rows.iter_mut() {
        t.on_top = t.unread || was.contains(&t.id);
    }
    rows.sort_by_key(|t| !t.on_top);
}

#[cfg(test)]
#[path = "ontop_tests.rs"]
mod tests;

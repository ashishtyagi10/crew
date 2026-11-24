//! The nav's reading of the mailbox (`inboxglance`), apart from the
//! pane's calls (`api`).
use super::super::query;
use super::super::wire::{self, Thread};
use super::Gmail;

impl Gmail {
    /// The nav's reading: unread threads, and the newest `n` unread ones —
    /// the INBOX card shows the first, the Inbox section all of them
    /// (the unread count, one list call, then one metadata read per thread
    /// in a single batch — a small share of the 6,000 units a minute).
    pub(crate) fn peek(&mut self, n: usize) -> Result<(u64, Vec<Thread>), String> {
        let unread = self.unread()?;
        let q = query::enc("in:inbox is:unread");
        let (ids, _) = wire::thread_ids(&self.get(&format!("/threads?maxResults={n}&q={q}"))?);
        if ids.is_empty() {
            return Ok((unread, Vec::new()));
        }
        let meta = "format=metadata&metadataHeaders=From&metadataHeaders=Subject";
        let reqs: Vec<_> = ids
            .iter()
            .map(|id| (format!("/threads/{id}?{meta}"), None))
            .collect();
        let rows = self.send_all(&reqs)?.iter().map(wire::thread).collect();
        Ok((unread, rows))
    }
}

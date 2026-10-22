//! The nav card's reading of the mailbox (`inboxglance`), apart from the
//! pane's calls (`api`).
use super::super::query;
use super::super::wire::{self, Thread};
use super::Gmail;

impl Gmail {
    /// The nav card's reading: unread threads, and the newest unread one
    /// (1 + 10 + 40 units — a full page is about 1,000).
    pub(crate) fn peek(&mut self) -> Result<(u64, Option<Thread>), String> {
        let unread = self.unread()?;
        let q = query::enc("in:inbox is:unread");
        let (ids, _) = wire::thread_ids(&self.get(&format!("/threads?maxResults=1&q={q}"))?);
        let Some(id) = ids.first() else {
            return Ok((unread, None));
        };
        let meta = "format=metadata&metadataHeaders=From&metadataHeaders=Subject";
        Ok((
            unread,
            Some(wire::thread(&self.get(&format!("/threads/{id}?{meta}"))?)),
        ))
    }
}

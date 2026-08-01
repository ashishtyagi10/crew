//! The reply-landing path: one settled `Message` from the broker replaces
//! that agent's provisional streaming card (inheriting its fold state) and
//! claims the usage stash its `Stats` event left behind (see
//! `chatflow::absorb_stats`). Split from `chatflow` for the 200-line cap.
use crate::chatflow::stream_key;
use crate::chatlayout::Message;

impl crate::chat::ChatPane {
    /// A settled reply arrived from `sender`: drop that agent's provisional
    /// card so the real `Message` takes its place. Any fragment the broker's
    /// gate swallowed is healed by the replacement. Returns whether the
    /// discarded card had been clicked open — the settled row inherits it,
    /// so a card expanded mid-stream doesn't snap shut on settling.
    pub(crate) fn settle_stream(&mut self, sender: &str) -> bool {
        let name = stream_key(sender);
        let mut expanded = false;
        self.streaming.retain(|m| {
            let same = stream_key(&m.sender) == name;
            expanded |= same && m.expanded;
            !same
        });
        expanded
    }

    /// One settled `Message` landed: replace the streaming card (keeping its
    /// fold state), close the hop, count unread, and attach the stashed usage
    /// — but only when the stat named THIS sender. The broker emits each
    /// reply's stat immediately before the reply itself, but adjacency is its
    /// behavior, not a contract: a message from anyone else must neither wear
    /// the trailer nor clear the stash (the matching reply may be next).
    pub(crate) fn absorb_message(
        &mut self,
        sender: String,
        text: String,
        ts: String,
        meta: String,
    ) {
        let expanded = self.settle_stream(&sender);
        self.awaiting = false; // a reply landed
        self.note_reply(&sender);
        if self.scroll > 0 {
            self.unread += 1; // arrived out of view
        }
        // Senders arrive as `"coder → user"` (relay hops) or bare `"coder"`;
        // `stream_key` normalizes both to the from-part — the exact same key
        // the streaming cards match on, so no looser match is needed.
        let usage = match &self.pending_reply_usage {
            Some((agent, ..)) if stream_key(&sender) == agent => self
                .pending_reply_usage
                .take()
                .map(|(_, tok_in, tok_out, cost)| (tok_in, tok_out, cost)),
            _ => None,
        };
        self.push_capped(Message {
            sender,
            text,
            ts,
            meta,
            usage,
            expanded,
        });
    }
}

#[cfg(test)]
#[path = "chatsettle_tests.rs"]
mod tests;

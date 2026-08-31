//! Keeping the crew pane's transcript: appending a card under the cap,
//! the system-voice note, and the per-task tally that rides with it.
//!
//! Split from [`crate::chatdrain`] for the line cap, along the line between
//! draining the broker and deciding what the transcript keeps.
use crate::chat::ChatPane;
use crate::chatlayout::Message;

impl ChatPane {
    /// A background task started or ended in the broker. Kept as a list of
    /// ids, oldest first, because `/stop #n` names one — and an END for a task
    /// this pane never saw start (a broker that outlived a reconnect) is a
    /// no-op rather than a phantom or a panic.
    pub(crate) fn absorb_task(&mut self, id: u64, running: bool) {
        self.running_tasks.retain(|t| *t != id);
        if running {
            self.running_tasks.push(id);
        }
    }

    /// Append a local "agent smith" note to the transcript — composer intercepts
    /// (`/theme`, `/export`) and app-side command echoes (`/font`) share it.
    pub(crate) fn push_note(&mut self, text: String) {
        self.messages.push(Message {
            sender: "agent smith".into(),
            text,
            ts: chrono::Local::now().timestamp_millis().to_string(),
            meta: String::new(),
            usage: None,
            expanded: false,
        });
    }

    /// Push `m` onto the transcript, then trim from the front to the
    /// 500-message cap. Shared by every site that appends to `messages` (the
    /// plugin `Message` arm here, a folded swarm block in `chatswarm.rs`, and
    /// the Esc-interrupt note below) so the cap can't drift out of sync
    /// between them.
    pub(crate) fn push_capped(&mut self, m: Message) {
        self.messages.push(m);
        if self.messages.len() <= Self::TRANSCRIPT_CAP {
            return;
        }
        // Fold with a marker rather than draining in silence — the cap has
        // always been automatic; what it lacked was honesty.
        //
        // Down to CAP - SLACK in one batch, not to CAP: folding to exactly
        // the cap means the next message folds again, and every message
        // after that, each pass copying the whole transcript. One pass per
        // SLACK messages instead.
        let mut msgs = std::mem::take(&mut self.messages);
        if self.folded > 0 {
            // The leading marker is not one of the messages being counted.
            msgs.remove(0);
        }
        let (kept, total) = crate::chatcompact::compact_messages(
            msgs,
            Self::TRANSCRIPT_CAP - Self::FOLD_SLACK,
            self.folded,
        );
        self.messages = kept;
        self.folded = total;
    }
}

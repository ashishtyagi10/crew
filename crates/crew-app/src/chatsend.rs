//! Sending, and stopping: what leaves the composer for the broker, and what
//! happens to a turn you interrupt.
//!
//! Split out of [`crate::chat`] for the line cap.
use crate::chat::ChatPane;
use crate::chatlayout::Message;
use crew_plugin::PluginCommand;

impl ChatPane {
    /// Send `text` to the broker on the pane's channel now, latching
    /// `awaiting` so the busy sweep runs until the reply lands. Shared by a
    /// direct send and a queue flush — both are "the broker gets this text
    /// now", just reached from different callers.
    pub(crate) fn send_now(&mut self, text: String) {
        let cmd = PluginCommand::Send {
            channel: self.channel.clone(),
            text,
        };
        match self.plugin.send(&cmd) {
            Ok(()) => self.awaiting = true, // wait for the reply
            Err(e) => eprintln!("crew-app: plugin send error: {e}"),
        }
    }

    /// Submit `text` to the broker as if it were typed in the composer: queued
    /// while the pane is busy (except `/stop`), sent immediately when idle —
    /// the same rule the composer's own Enter uses. Lets app-level commands
    /// that target this pane (e.g. the `/model` picker) reach the broker
    /// without a synthetic keystroke path.
    pub(crate) fn submit_command(&mut self, text: String) {
        if text.is_empty() {
            return;
        }
        if self.is_busy() && !crate::chatqueue::is_stop(&text) {
            self.queued.push_back(text);
        } else {
            self.send_now(text);
        }
    }

    /// Throw away everything queued behind the run, returning what to say
    /// about it — `None` when there was nothing waiting.
    ///
    /// Pushes its own note for the paths that have no note of their own (a
    /// typed `/stop`); [`Self::interrupt`] folds the returned phrase into
    /// its own line instead, so one cancel never reads as two events.
    pub(crate) fn drop_queue(&mut self) -> Option<String> {
        let n = self.queued.len();
        if n == 0 {
            return None;
        }
        self.queued.clear();
        Some(format!(
            "dropped {n} queued message{}",
            if n == 1 { "" } else { "s" }
        ))
    }

    /// Esc while the crew is busy and connected: cancel the in-flight run by
    /// sending `/stop` straight to the broker — bypassing the queue exactly
    /// like the composer's own `/stop` does (`send_now`, not `queued.push`),
    /// since it must reach the broker mid-turn to cancel it — and note the
    /// action in the transcript. Repeat Esc while still busy resends `/stop`
    /// (the broker's cancel is an idempotent `AtomicBool`) but the note is
    /// deduped: only pushed when the last transcript message isn't already
    /// this same note.
    pub(crate) fn interrupt(&mut self) {
        // Literal "/stop", not `chatmention::expand` — there's nothing to
        // expand in a fixed cancel token, so this stays allocation-free.
        self.send_now("/stop".to_string());
        // Anything typed while the crew was busy is waiting to be sent the
        // moment it goes idle — which cancelling is precisely what makes it
        // do. Left alone, Esc would STOP one run and immediately START every
        // follow-up queued behind it, each written on the premise that the
        // interrupted work was going fine.
        let note = match self.drop_queue() {
            None => Self::INTERRUPT_NOTE.to_string(),
            Some(dropped) => format!("{} \u{2014} {dropped}", Self::INTERRUPT_NOTE),
        };
        // Deduped against the note ABOUT to be pushed, not a fixed string: a
        // second Esc has nothing left to drop and so writes the plain note,
        // which is the one already sitting there.
        let already_noted = self
            .messages
            .last()
            .is_some_and(|m| m.sender == "agent smith" && m.text == note);
        if already_noted {
            return;
        }
        if self.scroll > 0 {
            self.unread += 1;
        }
        self.push_capped(Message {
            sender: "agent smith".into(),
            text: note,
            ts: String::new(),
            meta: String::new(),
            usage: None,
            expanded: false,
        });
    }
}

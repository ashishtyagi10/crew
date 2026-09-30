//! Steer, don't queue: a message typed while the crew works is OFFERED to the
//! running task, not only held until the whole turn ends.
//!
//! WHY: "also check the tests" or "no, the other file" waited in the queue
//! while the agent went on answering the wrong thing, and was sent after it
//! had. Now the composer also sends it to the broker as a
//! [`PluginCommand::Steer`], and the running task's tool loop folds it into
//! its next round (the broker's `steer`), saying so with a
//! [`crew_plugin::PluginEvent::Steered`] that lands before the answer.
//!
//! The queued copy stays until then, exactly as before: a task with no round
//! left to take it (a plain answer, a CLI agent) never says it was taken, and
//! the copy is sent when the pane goes idle. A swarm takes it too: every one
//! of its workers reads it at its next round (`crew_hive::steers`). Nothing is lost and
//! nothing is sent twice — the only change is that the message can arrive
//! EARLY.
use crate::chat::ChatPane;
use crate::chatlayout::Message;
use crew_plugin::PluginCommand;

/// The note under a message the running task took, quoting its first line.
pub(crate) const JOINED: &str = "\u{21b3} joined the running task";

/// Characters of the message the note quotes before it clips.
const QUOTE: usize = 60;

/// Whether `text` may be offered to the running task: plain words for the
/// agent at work. A slash command, a `#note` and an `@agent` dial are each a
/// thing of their own for the broker to route, so they are only queued.
pub(crate) fn steers(text: &str) -> bool {
    let t = text.trim_start();
    !t.is_empty() && !t.starts_with(['/', '#', '@'])
}

/// The joined note for `text`: [`JOINED`], then the message's first line.
/// Quoted because several may be waiting, and the note lands where the task
/// took it — under whatever it streamed since — not under the message.
pub(crate) fn joined_note(text: &str) -> String {
    let first = text.trim().lines().next().unwrap_or("").trim();
    let quote: String = first.chars().take(QUOTE).collect();
    let more = if first.chars().count() > QUOTE {
        "\u{2026}"
    } else {
        ""
    };
    format!("{JOINED}: \u{201c}{quote}{more}\u{201d}")
}

impl ChatPane {
    /// A composed message, on its way: sent at once when idle (or a `/stop`,
    /// which must reach a busy broker to cancel it); queued while busy — and,
    /// when it is plain words on a live connection, offered to the running
    /// task as well.
    ///
    /// Not while a plan waits for its answer: the broker's plan gate reads a
    /// bare `yes` as the decision, and a steer would carry it into a tool
    /// round instead.
    pub(crate) fn submit_typed(&mut self, text: String) {
        if !self.is_busy() || crate::chatqueue::is_stop(&text) {
            return self.send_now(text);
        }
        if self.connected && !self.plan_pending && steers(&text) {
            // A failed write is left to the queued copy, which the next send
            // reports if the pipe is really gone.
            let _ = self.plugin.send(&PluginCommand::Steer {
                channel: self.channel.clone(),
                text: text.clone(),
            });
        }
        self.queued.push_back(text);
    }

    /// The running task took `text`: its queued copy goes (the first equal
    /// one — the same words typed twice are two messages), and the
    /// transcript says it joined. The user's own card was echoed when it was
    /// typed, so the note is all that is added. Noted even when no copy is
    /// left — taken back with Backspace after the offer went out — because
    /// the agent did see it, and the pane must not pretend otherwise.
    pub(crate) fn absorb_steered(&mut self, text: String) {
        if let Some(i) = self.queued.iter().position(|q| *q == text) {
            self.queued.remove(i);
        }
        if self.scroll > 0 {
            self.unread += 1;
        }
        self.push_capped(Message {
            sender: "agent smith".into(),
            text: joined_note(&text),
            ts: chrono::Local::now().timestamp_millis().to_string(),
            meta: String::new(),
            usage: None,
            expanded: false,
        });
    }
}

#[cfg(test)]
#[path = "chatsteer_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "chatsteerdraw_tests.rs"]
mod draw_tests;

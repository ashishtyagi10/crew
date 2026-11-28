//! Rewind's memory of what you sent, and which task each message started.
//!
//! A message is echoed into the transcript when typed (a [`Mark`]), may wait in
//! the queue, and is sent later; the broker answers with a task only if the
//! send became one. A label cannot join the two: the same words sent twice,
//! the first refused, would bind the SECOND task to the FIRST message and cut
//! the transcript in the wrong place. So `send_now` numbers each send, the
//! number rides to the broker and comes back on the task's start, and a mark
//! binds only to the task that carries its own number.
//!
//! A mark that never reached the broker (pulled back, cancelled, steered into
//! the running task) is dropped on the spot; one the broker never made a task
//! of (refused, answered inline) is dropped when a later task acknowledges.
use crate::chat::ChatPane;
use crate::chatlayout::Message;

/// Sent messages remembered; older ones fall off the front.
const MARKS_MAX: usize = 100;

/// A message the user sent, and the task it started once the broker says so.
#[derive(Debug, Clone)]
pub(crate) struct Mark {
    pub ts: String,
    pub text: String,
    pub task: Option<u64>,
    /// The number it was sent with; 0 while it has not left (it may be queued).
    pub seq: u64,
}

impl Mark {
    /// Whether `sent` is this message, possibly with file bodies appended
    /// (`chatmention::expand`).
    fn made(&self, sent: &str) -> bool {
        let mine = self.text.trim();
        !mine.is_empty() && sent.trim().starts_with(mine)
    }

    fn unsent(&self) -> bool {
        self.seq == 0 && self.task.is_none()
    }
}

/// The confirmation on screen: what going back to `task` would change.
#[derive(Debug, Clone, Default)]
pub(crate) struct Ask {
    pub task: u64,
    pub files: u32,
    /// The first few paths, the ones edited outside crew first.
    pub names: Vec<String>,
    /// Which of `names` were edited outside crew.
    pub yours: Vec<String>,
    /// Some file was edited outside crew.
    pub outside: bool,
    /// Edits outside crew cannot be ruled out.
    pub unsure: bool,
    /// The broker's fingerprint of this list; the confirm hands it back.
    pub stamp: String,
}

#[derive(Debug, Default)]
pub(crate) struct Rewind {
    pub marks: Vec<Mark>,
    /// The last number handed to a send.
    pub sent: u64,
    /// The open list's selection (an index into [`Rewind::picks`]).
    pub sel: Option<usize>,
    pub ask: Option<Ask>,
    /// The message picked, until the broker's answer comes back.
    pub want: Option<Mark>,
}

impl Rewind {
    /// The messages that started a task, newest first.
    pub(crate) fn picks(&self) -> Vec<&Mark> {
        self.marks
            .iter()
            .rev()
            .filter(|m| m.task.is_some())
            .collect()
    }
}

impl ChatPane {
    /// Remember a message the user sent (`push_capped` calls this).
    pub(crate) fn note_sent(&mut self, m: &Message) {
        if m.sender != "user" {
            return;
        }
        let marks = &mut self.rewind.marks;
        marks.push(Mark {
            ts: m.ts.clone(),
            text: m.text.clone(),
            task: None,
            seq: 0,
        });
        let over = marks.len().saturating_sub(MARKS_MAX);
        marks.drain(..over);
    }

    /// The number for the send of `text` now leaving (`send_now`): the oldest
    /// message still waiting that it is — the queue is first in, first out.
    /// 0 for a send that has no message (a command the app made itself).
    pub(crate) fn rewind_stamp(&mut self, text: &str) -> u64 {
        let r = &mut self.rewind;
        let Some(m) = r.marks.iter_mut().find(|m| m.unsent() && m.made(text)) else {
            return 0;
        };
        r.sent += 1;
        m.seq = r.sent;
        m.seq
    }

    /// A send that never reached the broker: its mark goes.
    pub(crate) fn rewind_unsent(&mut self, seq: u64) {
        self.rewind.marks.retain(|m| seq == 0 || m.seq != seq);
    }

    /// A queued message left the queue without being sent (taken back into
    /// the composer, or joined to the running task): its mark goes. `newest`
    /// picks the last waiting copy rather than the first.
    pub(crate) fn rewind_unqueued(&mut self, text: &str, newest: bool) {
        let marks = &mut self.rewind.marks;
        let waiting = |m: &Mark| m.unsent() && m.made(text);
        let at = if newest {
            marks.iter().rposition(waiting)
        } else {
            marks.iter().position(waiting)
        };
        if let Some(i) = at {
            marks.remove(i);
        }
    }

    /// The whole queue was thrown away: so are the marks waiting in it.
    pub(crate) fn rewind_unqueue_all(&mut self) {
        self.rewind.marks.retain(|m| !m.unsent());
    }

    /// The broker started task `id` for the send numbered `seq`.
    pub(crate) fn rewind_bind(&mut self, seq: u64, id: u64) {
        if seq == 0 {
            return; // an old broker, or a send with no message: bind nothing
        }
        let marks = &mut self.rewind.marks;
        let Some(m) = marks.iter_mut().find(|m| m.seq == seq) else {
            return;
        };
        m.task = Some(id);
        // Sends are answered in order: any earlier one with no task by now
        // was refused or answered inline, and can never become one.
        marks.retain(|m| m.task.is_some() || m.seq == 0 || m.seq > seq);
    }
}

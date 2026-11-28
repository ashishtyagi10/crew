//! Rewind, pane side: Ctrl+B on an idle, empty composer lists the messages
//! you sent that started a task, newest first. Enter on one asks the broker
//! what going back would cost (`broker::rewind`), a one-line confirmation
//! names it, and a yes puts the files back, forgets the conversation after
//! that message, and puts its text in the composer to edit and send again.
//!
//! WHY not Esc-Esc, Claude Code's gesture: Esc here already means interrupt
//! while busy and CLOSE THE PANE once idle, so the first press of a double
//! would close it. Ctrl+B ("back") is free, and is the same chord off a Mac.
//!
//! WHY the transcript is cut, not dimmed: dimmed history still reads as
//! context, and the model no longer has it. The cut transcript is exactly
//! what the broker's thread now holds, and a one-line note says where it went.
//! A message is bound to its task by the number it was sent with, which the
//! broker hands back on the task's start (`chatrewindmarks`), so slash
//! commands answered inline never appear and the same words sent twice are
//! never confused.

use crate::chat::ChatPane;
use crate::chatkeys::{ChatAction, ChatInput};

/// Why a rewind will not start: going back would run the queued messages
/// against a conversation they were not written for, or silently lose them.
const QUEUED: &str = "rewind: send or clear the queued messages first";
/// Chars of a message shown in the list and the confirmation.
pub(crate) const SHOWN: usize = 44;

/// One line of `text`, clipped.
pub(crate) fn shown(text: &str) -> String {
    let line = text.lines().next().unwrap_or("").trim();
    match line.char_indices().nth(SHOWN) {
        Some((i, _)) => format!("{}\u{2026}", &line[..i]),
        None => line.to_string(),
    }
}

impl ChatPane {
    /// Ctrl+B and the keys of the list and the confirmation, which are modal.
    pub(crate) fn rewind_key(&mut self, k: &ChatInput) -> Option<Option<ChatAction>> {
        if let Some(a) = self.rewind.ask.clone() {
            match k {
                ChatInput::Enter if !self.queued.is_empty() => {
                    self.rewind.want = None;
                    self.push_note(QUEUED.into());
                }
                ChatInput::Enter => self.rewind_send(a.task, (true, &a.stamp)),
                ChatInput::Close => self.rewind.want = None,
                _ => return Some(None),
            }
            self.rewind.ask = None;
        } else if let Some(sel) = self.rewind.sel {
            let n = self.rewind.picks().len();
            match k {
                ChatInput::Up => self.rewind.sel = Some(sel.saturating_sub(1)),
                ChatInput::Down => self.rewind.sel = Some((sel + 1).min(n.saturating_sub(1))),
                ChatInput::Enter => self.rewind_pick(sel),
                ChatInput::Close => self.rewind.sel = None,
                _ => {}
            }
        } else if matches!(k, ChatInput::Rewind) {
            self.rewind_open();
        } else {
            return None;
        }
        Some(None)
    }

    fn rewind_open(&mut self) {
        if !self.input.is_empty() {
            return self.push_note("rewind needs an empty composer".into());
        }
        if !self.connected {
            return;
        }
        if self.is_busy() {
            return self.push_note("rewind: stop it first (/stop)".into());
        }
        if !self.queued.is_empty() {
            return self.push_note(QUEUED.into());
        }
        if self.rewind.picks().is_empty() {
            return self.push_note("rewind: no message of yours has started a task yet".into());
        }
        self.rewind.sel = Some(0);
    }

    fn rewind_pick(&mut self, sel: usize) {
        self.rewind.sel = None;
        let Some(m) = self.rewind.picks().get(sel).map(|m| (*m).clone()) else {
            return;
        };
        if self.is_busy() {
            return self.push_note("rewind: stop it first (/stop)".into());
        }
        if !self.queued.is_empty() {
            return self.push_note(QUEUED.into());
        }
        if let Some(task) = m.task {
            self.rewind.want = Some(m);
            self.rewind_send(task, (false, ""));
        }
    }
}

#[cfg(test)]
#[path = "chatrewind_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "chatrewindfix_tests.rs"]
mod fix;
#[cfg(test)]
#[path = "chatrewindkeys_tests.rs"]
mod keys_tests;

#[cfg(test)]
#[path = "chatrewindsend_tests.rs"]
mod send_tests;

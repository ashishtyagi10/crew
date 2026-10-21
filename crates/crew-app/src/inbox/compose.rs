//! `r` in the reader: a reply box under the open mail, written and sent
//! right there (the user, 2026-10-21: "'r' should open a reply box rather
//! than agent typing the response in the smith pane"). To the last person
//! who wrote, on the thread's subject, threaded as Gmail threads it.
//! Cmd+Enter (Ctrl+Enter off a Mac) sends; Esc closes it, a second Esc when
//! there are words; Tab asks agent smith to write a draft into the box,
//! which is yours to change before it goes.
use winit::keyboard::{Key, NamedKey};

use super::draft::{self, Answer};
use super::worker::Ask;
use super::InboxPane;

pub(crate) struct Compose {
    pub answer: Answer,
    pub text: String,
    /// The caret, in chars.
    pub at: usize,
    /// Esc pressed once over words: a second discards them.
    pub armed: bool,
    /// Sent, and waiting for Gmail to say so: the box stays until it does.
    pub sending: bool,
}

impl InboxPane {
    pub(crate) fn composing(&self) -> bool {
        self.compose.is_some()
    }

    /// `r`: the box, addressed to the open thread. The line to say.
    pub(crate) fn open_compose(&mut self) -> &'static str {
        let Some(r) = self.reading.as_ref() else {
            return "";
        };
        let mails = r.mails.as_deref().unwrap_or_default();
        let Some(answer) = draft::answer(&r.thread.id, mails, &self.email) else {
            return "nothing in this thread to reply to";
        };
        if self.compose.is_none() {
            let (text, at, armed, sending) = (String::new(), 0, false, false);
            self.compose = Some(Compose {
                answer,
                text,
                at,
                armed,
                sending,
            });
        }
        ""
    }

    /// A key while the box is open: every one is the box's.
    pub(crate) fn compose_key(&mut self, key: &Key) {
        let Some(c) = self.compose.as_mut() else {
            return;
        };
        let armed = std::mem::take(&mut c.armed);
        let len = c.text.chars().count();
        let byte = |t: &str, at: usize| t.char_indices().nth(at).map_or(t.len(), |(b, _)| b);
        match key {
            Key::Named(NamedKey::Escape) if armed || c.text.trim().is_empty() => {
                self.compose = None;
                self.note = String::new();
            }
            Key::Named(NamedKey::Escape) => {
                c.armed = true;
                self.note = "esc again discards your reply".into();
            }
            Key::Named(NamedKey::Tab) => {
                self.act(super::handoff::Then::Draft);
                self.note = "agent smith is writing a draft into the box\u{2026}".into();
            }
            Key::Named(NamedKey::Backspace) if c.at > 0 => {
                let b = byte(&c.text, c.at - 1);
                c.text.remove(b);
                c.at -= 1;
            }
            Key::Named(NamedKey::Delete) if c.at < len => {
                let b = byte(&c.text, c.at);
                c.text.remove(b);
            }
            Key::Named(NamedKey::ArrowLeft) => c.at = c.at.saturating_sub(1),
            Key::Named(NamedKey::ArrowRight) => c.at = (c.at + 1).min(len),
            Key::Named(NamedKey::Home) => c.at = 0,
            Key::Named(NamedKey::End) => c.at = len,
            Key::Named(NamedKey::Enter) => self.type_in("\n"),
            Key::Named(NamedKey::Space) => self.type_in(" "),
            Key::Character(s) => self.type_in(s),
            _ => {}
        }
    }

    /// `text` at the caret: a key, a paste, agent smith's draft.
    pub(crate) fn type_in(&mut self, text: &str) {
        let Some(c) = self.compose.as_mut().filter(|c| !c.sending) else {
            return;
        };
        let clean: String = text
            .replace("\r\n", "\n")
            .chars()
            .filter(|&ch| ch == '\n' || !ch.is_control())
            .collect();
        let b = c
            .text
            .char_indices()
            .nth(c.at)
            .map_or(c.text.len(), |(b, _)| b);
        c.text.insert_str(b, &clean);
        c.at += clean.chars().count();
    }

    /// Cmd+Enter: the reply to Gmail, in its thread. The box stays until
    /// Gmail says it went, so words that failed to go are still there.
    pub(crate) fn send_reply(&mut self) {
        let Some(c) = self.compose.as_mut() else {
            return;
        };
        if c.sending {
            return;
        }
        if c.text.trim().is_empty() {
            self.note = "write something first".into();
            return;
        }
        c.sending = true;
        let raw = draft::raw(&c.answer, &c.text);
        let (thread_id, to) = (c.answer.thread_id.clone(), c.answer.to.clone());
        self.note = format!("sending to {}\u{2026}", super::wire::sender_name(&to));
        self.ask(Ask::Send { thread_id, raw, to });
    }
}

#[cfg(test)]
#[path = "compose_tests.rs"]
mod tests;

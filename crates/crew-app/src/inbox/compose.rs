//! `r` in the reader: a reply box under the open mail, written and sent
//! right there (the user, 2026-10-21: "'r' should open a reply box rather
//! than agent typing the response in the smith pane"). To everyone on the
//! mail (the user: "reply -- should always be reply all"), on the thread's
//! subject, threaded as Gmail threads it; `f` forwards it instead (the user:
//! "add cc and forward to the reply box", then "an attach-a-file option"):
//! the box has `to` and `cc` lines to change and a row of files
//! (`composekeys` moves between them, `attach` fills the row). Cmd+Enter
//! (Ctrl+Enter off a Mac) sends (`composesend`); Esc closes it, a second Esc
//! when there are words.
use std::sync::mpsc::Receiver;

use super::attach::Attached;
use super::draft::{self, Answer};
use super::field::Field;
use super::forward::{self, Forwarded};
use super::InboxPane;

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Kind {
    /// To everyone the mail went to.
    Reply,
    Forward,
}

/// Where the caret is: a line of addresses, the words, or the files.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum On {
    To,
    Cc,
    Body,
    Files,
}

pub(crate) struct Compose {
    pub kind: Kind,
    /// The headers it opened with: the subject and threading are kept, the
    /// addresses are what the `to` and `cc` lines say when it goes.
    pub answer: Answer,
    pub to: Field,
    pub cc: Field,
    pub body: Field,
    pub on: On,
    /// Files of yours sent with it, and the one chosen on their row
    /// (`files.len()`: the row's `+ attach`).
    pub files: Vec<Attached>,
    pub pick: usize,
    /// The system's file chooser, open until it answers.
    pub picking: Option<Receiver<Result<Vec<std::path::PathBuf>, String>>>,
    /// The mail a forward carries.
    pub forward: Option<Box<Forwarded>>,
    /// Esc pressed once over words: a second discards them.
    pub armed: bool,
    /// Sent, and waiting for Gmail to say so: the box stays until it does.
    pub sending: bool,
}

impl Compose {
    /// The place being typed in (the row of files types nothing, and
    /// `composekeys` never edits it).
    pub(crate) fn field(&mut self) -> &mut Field {
        match self.on {
            On::To => &mut self.to,
            On::Cc => &mut self.cc,
            On::Body | On::Files => &mut self.body,
        }
    }

    /// Nothing written or changed since it opened: Esc lets it go at once.
    pub(crate) fn untouched(&self) -> bool {
        self.body.text.trim().is_empty()
            && self.to.text == self.answer.to
            && self.cc.text == self.answer.cc
            && self.files.is_empty()
    }
}

impl InboxPane {
    pub(crate) fn composing(&self) -> bool {
        self.carded()
    }

    /// `r` or `f`: the box, for the open thread. The line to say.
    pub(crate) fn open_compose(&mut self, kind: Kind) -> &'static str {
        let Some(r) = self.reading.as_ref() else {
            return "";
        };
        if self.compose.is_some() {
            return "";
        }
        let mails = r.mails.as_deref().unwrap_or_default();
        let (answer, forward) = match kind {
            Kind::Reply => (draft::answer_all(&r.thread.id, mails, &self.email), None),
            Kind::Forward => match mails.last() {
                Some(m) => {
                    let subject = match m.reply.subject.trim() {
                        "" => r.thread.subject.as_str(),
                        s => s,
                    };
                    let a = Answer {
                        subject: forward::subject(subject),
                        ..Answer::default()
                    };
                    (Some(a), Some(Box::new(Forwarded::of(m))))
                }
                None => (None, None),
            },
        };
        let Some(answer) = answer else {
            return "nothing in this thread to reply to";
        };
        let on = match kind {
            Kind::Forward => On::To,
            _ => On::Body,
        };
        self.compose = Some(Compose {
            kind,
            to: Field::with(&answer.to),
            cc: Field::with(&answer.cc),
            answer,
            body: Field::default(),
            on,
            files: Vec::new(),
            pick: 0,
            picking: None,
            forward,
            armed: false,
            sending: false,
        });
        ""
    }
}

#[cfg(test)]
#[path = "compose_tests.rs"]
mod tests;

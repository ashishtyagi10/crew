//! `r` in the reader: a reply box under the open mail, written and sent
//! right there (the user, 2026-10-21: "'r' should open a reply box rather
//! than agent typing the response in the smith pane"). To the last person
//! who wrote, on the thread's subject, threaded as Gmail threads it. `R`
//! replies to everyone on the mail, `f` forwards it (the user: "add cc and
//! forward to the reply box"): the box has a `to` and a `cc` line to change
//! (`composekeys` moves between them). Cmd+Enter (Ctrl+Enter off a Mac)
//! sends; Esc closes it, a second Esc when there are words.
use super::draft::{self, Answer};
use super::field::Field;
use super::forward::{self, Forwarded};
use super::worker::Ask;
use super::{addr, InboxPane};

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Kind {
    Reply,
    /// `R`: to everyone the mail went to.
    All,
    Forward,
}

/// Where the caret is: a line of addresses, or the words.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum On {
    To,
    Cc,
    Body,
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
    /// The mail a forward carries.
    pub forward: Option<Box<Forwarded>>,
    /// Esc pressed once over words: a second discards them.
    pub armed: bool,
    /// Sent, and waiting for Gmail to say so: the box stays until it does.
    pub sending: bool,
}

impl Compose {
    pub(crate) fn field(&mut self) -> &mut Field {
        match self.on {
            On::To => &mut self.to,
            On::Cc => &mut self.cc,
            On::Body => &mut self.body,
        }
    }

    /// Nothing written or changed since it opened: Esc lets it go at once.
    pub(crate) fn untouched(&self) -> bool {
        self.body.text.trim().is_empty()
            && self.to.text == self.answer.to
            && self.cc.text == self.answer.cc
    }
}

impl InboxPane {
    pub(crate) fn composing(&self) -> bool {
        self.compose.is_some()
    }

    /// `r`, `R` or `f`: the box, for the open thread. The line to say.
    pub(crate) fn open_compose(&mut self, kind: Kind) -> &'static str {
        let Some(r) = self.reading.as_ref() else {
            return "";
        };
        if self.compose.is_some() {
            return "";
        }
        let mails = r.mails.as_deref().unwrap_or_default();
        let (answer, forward) = match kind {
            Kind::Reply => (draft::answer(&r.thread.id, mails, &self.email), None),
            Kind::All => (draft::answer_all(&r.thread.id, mails, &self.email), None),
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
            forward,
            armed: false,
            sending: false,
        });
        ""
    }

    /// Cmd+Enter: to Gmail, a reply in its thread, a forward on its own.
    /// The box stays until Gmail says it went, so words that failed to go
    /// are still there.
    pub(crate) fn send_reply(&mut self) {
        let Some(c) = self.compose.as_mut().filter(|c| !c.sending) else {
            return;
        };
        let refused = if addr::split(&c.to.text).is_empty() {
            c.on = On::To;
            Some("who is it to? write an address on the to line".to_string())
        } else if let Some((bad, on)) = addr::first_bad(&c.to.text)
            .map(|b| (b, On::To))
            .or_else(|| addr::first_bad(&c.cc.text).map(|b| (b, On::Cc)))
        {
            c.on = on;
            Some(format!("\u{201c}{bad}\u{201d} is not an address"))
        } else if c.body.text.trim().is_empty() && c.forward.is_none() {
            Some("write something first".to_string())
        } else {
            match c.forward.as_ref().map(|f| f.size()) {
                Some(n) if n > forward::MOST => Some(format!(
                    "its files come to {} MB, more than the 25 MB Gmail sends \u{2014} forward it from Gmail (o)",
                    n.div_ceil(1024 * 1024)
                )),
                _ => None,
            }
        };
        if let Some(why) = refused {
            self.note = why;
            return;
        }
        c.sending = true;
        let answer = Answer {
            to: addr::split(&c.to.text).join(", "),
            cc: addr::split(&c.cc.text).join(", "),
            ..c.answer.clone()
        };
        let to = answer.to.clone();
        self.note = format!("sending to {}\u{2026}", addr::names(&to));
        let ask = match c.forward.clone() {
            Some(of) => Ask::Forward {
                head: draft::head(&answer),
                note: c.body.text.clone(),
                of,
                to,
            },
            None => Ask::Send {
                thread_id: answer.thread_id.clone(),
                raw: draft::raw(&answer, &c.body.text),
                to,
            },
        };
        self.ask(ask);
    }
}

#[cfg(test)]
#[path = "compose_tests.rs"]
mod tests;

//! Cmd+Enter in the reply box: every address checked, then to Gmail. A
//! plain reply goes as it always did (`Ask::Send`); a forward, or anything
//! carrying files or saved as a draft, is built by the worker (`outgoing`).
use super::compose::{Compose, On};
use super::draft::{self, Answer};
use super::outgoing::Outgoing;
use super::worker::Ask;
use super::{addr, forward, InboxPane};

impl InboxPane {
    /// The box stays until Gmail says it went, so words that failed to go
    /// are still there.
    pub(crate) fn send_reply(&mut self) {
        let ready = self.ready();
        let Some(c) = self.compose.as_mut().filter(|c| !c.sending && ready) else {
            return;
        };
        let bad = [(On::To, &c.to), (On::Cc, &c.cc)]
            .into_iter()
            .find_map(|(on, f)| addr::first_bad(&f.text).map(|b| (b, on)));
        let size = c.forward.as_ref().map_or(0, |f| f.size())
            + c.files.iter().map(|f| f.size).sum::<u64>();
        let refused = if addr::split(&c.to.text).is_empty() {
            c.on = On::To;
            Some("who is it to? write an address on the to line".to_string())
        } else if let Some((bad, on)) = bad {
            c.on = on;
            Some(format!("\u{201c}{bad}\u{201d} is not an address"))
        } else if c.body.text.trim().is_empty() && c.forward.is_none() && c.files.is_empty() {
            Some("write something first".to_string())
        } else if size > forward::MOST {
            Some(format!(
                "its files come to {} MB, more than the 25 MB Gmail sends",
                size.div_ceil(1024 * 1024)
            ))
        } else {
            None
        };
        if let Some(why) = refused {
            self.note = why;
            return;
        }
        c.sending = true;
        let o = c.outgoing();
        self.note = format!("sending to {}\u{2026}", addr::names(&o.to));
        // A plain reply goes as it always did; anything else is built.
        let plain = c.forward.is_none() && c.files.is_empty() && c.draft.is_none();
        let ask = match plain {
            true => Ask::Send {
                thread_id: o.thread_id.clone().unwrap_or_default(),
                raw: draft::raw(&c.answer_now(), &c.body.text),
                to: o.to,
            },
            false => Ask::Mail(Box::new(o)),
        };
        self.ask(ask);
    }
}

impl Compose {
    /// The headers as the box has them now: its lines' addresses and, for a
    /// new mail, its subject; a reply's subject and threading are the thread's.
    pub(crate) fn answer_now(&self) -> Answer {
        let list = |f: &super::field::Field| addr::split(&f.text).join(", ");
        let subject = match self.kind {
            super::compose::Kind::New => self.subject.text.trim().to_string(),
            _ => self.answer.subject.clone(),
        };
        Answer {
            to: list(&self.to),
            cc: list(&self.cc),
            subject,
            ..self.answer.clone()
        }
    }

    /// The whole mail, for the worker to build: to send, or to save as a draft.
    pub(crate) fn outgoing(&self) -> Outgoing {
        let answer = self.answer_now();
        Outgoing {
            head: draft::head(&answer),
            thread_id: (self.forward.is_none() && !answer.thread_id.is_empty())
                .then(|| answer.thread_id.clone()),
            words: self.body.text.clone(),
            forward: self.forward.clone(),
            files: self.files.clone(),
            to: answer.to,
            draft: self.draft.clone(),
        }
    }
}

#[cfg(test)]
#[path = "composesend_tests.rs"]
mod tests;

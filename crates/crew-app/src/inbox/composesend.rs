//! Cmd+Enter in the reply box: every address checked, then to Gmail. A
//! plain reply goes as it always did (`Ask::Send`); a forward, or anything
//! carrying files, is built by the worker (`outgoing`).
use super::compose::On;
use super::draft::{self, Answer};
use super::outgoing::Outgoing;
use super::worker::Ask;
use super::{addr, forward, InboxPane};

impl InboxPane {
    /// The box stays until Gmail says it went, so words that failed to go
    /// are still there.
    pub(crate) fn send_reply(&mut self) {
        let Some(c) = self.compose.as_mut().filter(|c| !c.sending) else {
            return;
        };
        let bad = [(On::To, &c.to), (On::Cc, &c.cc), (On::Bcc, &c.bcc)]
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
        let list = |f: &super::field::Field| addr::split(&f.text).join(", ");
        let answer = Answer {
            to: list(&c.to),
            cc: list(&c.cc),
            bcc: list(&c.bcc),
            ..c.answer.clone()
        };
        let to = answer.to.clone();
        self.note = format!("sending to {}\u{2026}", addr::names(&to));
        let ask = match c.forward.is_none() && c.files.is_empty() {
            true => Ask::Send {
                thread_id: answer.thread_id.clone(),
                raw: draft::raw(&answer, &c.body.text),
                to,
            },
            false => Ask::Mail(Box::new(Outgoing {
                head: draft::head(&answer),
                thread_id: c.forward.is_none().then(|| answer.thread_id.clone()),
                words: c.body.text.clone(),
                forward: c.forward.clone(),
                files: c.files.clone(),
                to,
            })),
        };
        self.ask(ask);
    }
}

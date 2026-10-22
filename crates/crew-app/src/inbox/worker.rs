//! The inbox's network thread. The pane runs on the winit thread, so every
//! call to Google happens here and comes back as a [`Got`] the pane drains in
//! `poll`.
//!
//! Gmail charges each project's user 6,000 units a minute (projects made
//! since 2026-05-01), and one page of 25 threads costs about 1,011. So the
//! pane never re-lists on a timer: a [`Ask::Check`] reads the profile's
//! `historyId` (1 unit) and the INBOX label (1 unit), and only a mailbox
//! that moved since the last page is listed again.
use std::sync::mpsc::{self, Receiver, Sender};

use super::api::Gmail;
use super::grant::Grant;

pub(crate) use super::asks::{Ask, Got};

pub(crate) struct Worker {
    pub(crate) tx: Sender<Ask>,
    pub(crate) rx: Receiver<Got>,
}

pub(crate) fn start(grant: Grant) -> Worker {
    let (tx, asks) = mpsc::channel::<Ask>();
    let (out, rx) = mpsc::channel::<Got>();
    std::thread::spawn(move || {
        let mut g = match Gmail::new(grant) {
            Ok(g) => g,
            Err(e) => return drop(out.send(Got::Failed(e))),
        };
        let mut state = State::default();
        let _ = out.send(match g.profile() {
            Ok((email, _)) => Got::Account(email),
            Err(e) => fail(e),
        });
        while let Ok(ask) = asks.recv() {
            for got in state.answer(&mut g, ask) {
                if out.send(got).is_err() {
                    return; // the pane closed
                }
            }
        }
    });
    Worker { tx, rx }
}

/// What the worker remembers between asks.
#[derive(Default)]
struct State {
    query: String,
    next: Option<String>,
    /// Pages of `query` the pane holds, the first one included.
    pages: usize,
    history: String,
}

impl State {
    fn answer(&mut self, g: &mut Gmail, ask: Ask) -> Vec<Got> {
        match ask {
            Ask::List(q) => {
                self.query = q;
                self.pages = 0;
                self.list(g, None)
            }
            Ask::More => match self.next.clone() {
                Some(p) => self.list(g, Some(p)),
                None => Vec::new(),
            },
            Ask::Open(id) => vec![match g.open(&id) {
                Ok(m) => Got::Opened(id, m),
                Err(e) => failed_as(e, |e| Got::Unopened(id, e)),
            }],
            // The pane already shows the change; a refusal is said, and the
            // list comes back as Gmail has it, which puts the row right.
            Ask::Relabel { id, add, remove } => {
                let done = g.relabel(&id, &add, &remove);
                self.filed(g, done)
            }
            Ask::Trash { id, back } => {
                let done = g.trash(&id, back);
                self.filed(g, done)
            }
            Ask::Picture {
                key,
                msg,
                att,
                data,
            } => {
                vec![super::picfetch::carried(g, key, &msg, att, data)]
            }
            Ask::Save {
                msg,
                att,
                data,
                name,
            } => vec![super::save::save(g, &msg, att, data, &name)],
            Ask::Send { thread_id, raw, to } => {
                vec![g
                    .send(&thread_id, &raw)
                    .map_or_else(|e| failed_as(e, Got::NotSent), |()| Got::Sent(to))]
            }
            Ask::Mail(o) => {
                let went = super::outgoing::send(g, &o);
                let to = o.to.clone();
                vec![went.map_or_else(
                    |e| failed_as(e, Got::NotSent),
                    |()| match o.forward.is_some() {
                        true => Got::Forwarded(to),
                        false => Got::Sent(to),
                    },
                )]
            }
            Ask::Unsubscribe { how, who } => {
                vec![g
                    .unsubscribe(&how)
                    .map_or_else(fail, |()| Got::Unsubscribed(who))]
            }
            Ask::Check => match g.profile().map(|(_, h)| h) {
                Ok(h) if h == self.history => vec![Got::Checked],
                Ok(_) => self.list(g, None),
                Err(e) => vec![fail(e)],
            },
        }
    }

    /// A filing call's answer: the unread count when it worked.
    fn filed(&mut self, g: &mut Gmail, done: Result<(), String>) -> Vec<Got> {
        match done {
            // The count the pane's name and the nav card say, at once
            // (labels.get, one unit); none when it could not be had.
            Ok(()) => g.unread().ok().map(Got::Unread).into_iter().collect(),
            Err(e) => std::iter::once(fail(e)).chain(self.list(g, None)).collect(),
        }
    }

    fn list(&mut self, g: &mut Gmail, from: Option<String>) -> Vec<Got> {
        let more = from.is_some();
        // Read before the page, so a mail landing mid-list moves it again.
        if let Ok((_, h)) = g.profile() {
            self.history = h;
        }
        let mut out = vec![g.unread().map_or_else(fail, Got::Unread)];
        out.push(match g.page(&self.query, from.as_deref()) {
            Ok((rows, next)) => {
                let whole = next.is_none();
                self.went(more, next);
                Got::Page {
                    query: self.query.clone(),
                    rows,
                    next: self.next.clone(),
                    more,
                    whole,
                }
            }
            Err(e) => fail(e),
        });
        out
    }

    /// Where `j` at the end of the list goes on from after a page: the page's
    /// own cursor, except that a first page listed again (the check after the
    /// mailbox moved) of a list already paged past it keeps the deeper one,
    /// as the pane keeps those rows. Gmail's end ends the list either way.
    fn went(&mut self, more: bool, next: Option<String>) {
        match (more, next) {
            (false, Some(_)) if self.pages > 1 => {}
            (more, next) => {
                self.pages = if more { self.pages + 1 } else { 1 };
                self.next = next;
            }
        }
    }
}

/// An `invalid_grant` is the grant's end, not one call's failure.
fn fail(e: String) -> Got {
    match e.starts_with("invalid_grant") {
        true => Got::Ended(e),
        false => Got::Failed(e),
    }
}

/// A call whose failure is its own answer (mail that did not go, a thread
/// that did not open), unless the grant ended with it.
fn failed_as(e: String, own: impl FnOnce(String) -> Got) -> Got {
    match fail(e) {
        Got::Failed(e) => own(e),
        ended => ended,
    }
}

#[cfg(test)]
#[path = "worker_tests.rs"]
mod tests;

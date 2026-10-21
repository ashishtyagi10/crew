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

use super::api::{Gmail, Mail};
use super::grant::Grant;
use super::wire::Thread;

pub(crate) enum Ask {
    /// The first page for `query` (empty = the inbox).
    List(String),
    /// The page after the last one.
    More,
    Open(String),
    /// Has anything changed? Re-list when the mailbox moved.
    Check,
    /// Save `raw` (RFC 2822, base64url) as a draft in the thread.
    Draft {
        thread_id: String,
        raw: String,
        subject: String,
    },
    /// Add and take away labels on a thread (archive, read, star).
    Relabel {
        id: String,
        add: Vec<&'static str>,
        remove: Vec<&'static str>,
    },
    /// Move a thread to Trash, or (`back`) out of it.
    Trash {
        id: String,
        back: bool,
    },
    /// A picture the mail carries: inline `data`, or attachment `att` of
    /// message `msg`, decoded here and sent back under `key`.
    Picture {
        key: String,
        msg: String,
        att: Option<String>,
        data: Option<String>,
    },
}

pub(crate) enum Got {
    /// The signed-in account.
    Account(String),
    Page {
        query: String,
        rows: Vec<Thread>,
        next: Option<String>,
        /// Rows to add to the list rather than replace it.
        more: bool,
    },
    Unread(u64),
    Opened(String, Vec<Mail>),
    /// A draft was saved; its subject.
    Drafted(String),
    /// A picture, decoded, or `None` when there is nothing to draw.
    Picture(String, Option<crate::viewpane::bitmap::Bitmap>),
    /// A call failed; the pane says why and keeps what it has.
    Failed(String),
    /// Google ended the grant (revoked, expired): sign in again.
    Ended(String),
}

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
    history: String,
}

impl State {
    fn answer(&mut self, g: &mut Gmail, ask: Ask) -> Vec<Got> {
        match ask {
            Ask::List(q) => {
                self.query = q;
                self.list(g, None)
            }
            Ask::More => match self.next.clone() {
                Some(p) => self.list(g, Some(p)),
                None => Vec::new(),
            },
            Ask::Open(id) => vec![g.open(&id).map_or_else(fail, |m| Got::Opened(id, m))],
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
            Ask::Draft {
                thread_id,
                raw,
                subject,
            } => {
                vec![g
                    .draft(&thread_id, &raw)
                    .map_or_else(fail, |()| Got::Drafted(subject))]
            }
            Ask::Check => match g.profile().map(|(_, h)| h) {
                Ok(h) if h == self.history => vec![],
                Ok(_) => self.list(g, None),
                Err(e) => vec![fail(e)],
            },
        }
    }

    /// A filing call's answer: nothing to say when it worked.
    fn filed(&mut self, g: &mut Gmail, done: Result<(), String>) -> Vec<Got> {
        match done {
            Ok(()) => vec![],
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
                self.next = next.clone();
                Got::Page {
                    query: self.query.clone(),
                    rows,
                    next,
                    more,
                }
            }
            Err(e) => fail(e),
        });
        out
    }
}

/// An `invalid_grant` is the grant's end, not one call's failure.
fn fail(e: String) -> Got {
    match e.starts_with("invalid_grant") {
        true => Got::Ended(e),
        false => Got::Failed(e),
    }
}

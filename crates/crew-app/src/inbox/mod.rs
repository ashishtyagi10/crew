//! `/inbox` — a light Gmail inbox in a pane (goal 2026-10-20-inbox).
//!
//! Read-only: the list, Gmail's own search, a thread read as text, and the
//! browser one key away for anything more. Nothing is stored on disk but the
//! grant (`grant`); the rows live in memory and go with the pane. The network
//! is a thread of its own (`worker`), and the pane only drains what it says.
pub(crate) mod age;
pub(crate) mod api;
pub(crate) mod body;
pub(crate) mod draw;
pub(crate) mod fetch;
pub(crate) mod grant;
pub(crate) mod keys;
pub(crate) mod query;
pub(crate) mod reader;
pub(crate) mod setup;
pub(crate) mod signin;
pub(crate) mod take;
pub(crate) mod triage;
pub(crate) mod wire;
pub(crate) mod worker;

use std::path::PathBuf;
use std::sync::mpsc::Receiver;

use api::Mail;
use grant::{Client, Grant};
use wire::Thread;
use worker::{Ask, Got, Worker};

/// How often a signed-in pane asks whether the mailbox moved (2 units).
const CHECK_MS: u64 = 60_000;
/// How often the setup page looks in Downloads for the client file.
const LOOK_MS: u64 = 2_000;

pub(crate) enum Stage {
    /// No grant yet: how to make a client, and the one found, if any.
    Setup {
        found: Option<(PathBuf, Client)>,
    },
    SigningIn(Receiver<Result<Grant, String>>),
    Ready(Worker),
}

/// An opened thread: its row, its messages once they arrive, and the scroll.
pub(crate) struct Reading {
    pub thread: Thread,
    pub mails: Option<Vec<Mail>>,
    pub scroll: usize,
}

pub struct InboxPane {
    pub(crate) stage: Stage,
    pub(crate) rows: Vec<Thread>,
    pub(crate) sel: usize,
    pub(crate) query: String,
    /// The search being typed, while `/` is open.
    pub(crate) typing: Option<String>,
    pub(crate) reading: Option<Reading>,
    pub(crate) unread: Option<u64>,
    pub(crate) email: String,
    /// One line of what just happened (an error, a sign-in), until the next.
    pub(crate) note: String,
    pub(crate) busy: bool,
    /// Gmail has another page after the rows shown.
    pub(crate) more: bool,
    /// The last archive, read or star, for `u`.
    pub(crate) undo: Option<triage::Undo>,
    ticked_ms: u64,
}

impl InboxPane {
    /// Signed in when a grant is on disk; else the setup page.
    pub fn new() -> Self {
        let stage = match grant::load() {
            Some(g) => Stage::Ready(worker::start(g)),
            None => Stage::Setup {
                found: setup::look(),
            },
        };
        let mut p = Self::with(stage);
        p.ask(Ask::List(String::new()));
        p
    }

    pub(crate) fn with(stage: Stage) -> Self {
        Self {
            stage,
            rows: Vec::new(),
            sel: 0,
            query: String::new(),
            typing: None,
            reading: None,
            unread: None,
            email: String::new(),
            note: String::new(),
            busy: false,
            more: false,
            undo: None,
            ticked_ms: crate::anim::now_ms(),
        }
    }

    /// Send `ask` to the worker; `false` when not signed in.
    pub(crate) fn ask(&mut self, ask: Ask) -> bool {
        let Stage::Ready(w) = &self.stage else {
            return false;
        };
        self.busy = !matches!(ask, Ask::Check);
        w.tx.send(ask).is_ok()
    }

    /// The card's legend: `inbox · 12 unread`, or the search shown.
    pub(crate) fn legend(&self) -> String {
        match (self.query.trim(), self.unread) {
            ("", Some(n)) if n > 0 => format!("inbox \u{b7} {n} unread"),
            ("", _) => "inbox".into(),
            (q, _) => format!("inbox \u{b7} {q}"),
        }
    }

    /// The nav's one-line glance.
    pub(crate) fn glance(&self) -> String {
        match (&self.stage, self.unread) {
            (Stage::Ready(_), Some(n)) => format!("{n} unread"),
            (Stage::Ready(_), None) => "checking\u{2026}".into(),
            _ => "not signed in".into(),
        }
    }

    /// Drain what came back; true when the pane should repaint.
    pub fn poll(&mut self) -> bool {
        let now = crate::anim::now_ms();
        let due = now.saturating_sub(self.ticked_ms) >= self.tick_ms();
        if due {
            self.ticked_ms = now;
        }
        match &mut self.stage {
            Stage::Setup { found } if due && found.is_none() => {
                *found = setup::look();
                found.is_some()
            }
            Stage::Setup { .. } => false,
            Stage::SigningIn(rx) => match rx.try_recv() {
                Ok(got) => self.signed(got),
                Err(_) => false,
            },
            Stage::Ready(w) => {
                let got: Vec<Got> = w.rx.try_iter().collect();
                let changed = !got.is_empty();
                for g in got {
                    self.take(g);
                }
                if due && !self.busy {
                    self.ask(Ask::Check);
                }
                changed
            }
        }
    }

    fn tick_ms(&self) -> u64 {
        match self.stage {
            Stage::Setup { .. } => LOOK_MS,
            _ => CHECK_MS,
        }
    }
}

impl Default for InboxPane {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;

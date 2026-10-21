//! `/inbox` — a light Gmail inbox in a pane (goal 2026-10-20-inbox).
//!
//! Read-only: the list, Gmail's own search, a thread read as text, and the
//! browser one key away for anything more. Nothing is stored on disk but the
//! grant (`grant`); the rows live in memory and go with the pane. The network
//! is a thread of its own (`worker`), and the pane only drains what it says.
pub(crate) mod age;
pub(crate) mod api;
mod asks;
pub(crate) mod body;
pub(crate) mod charset;
pub(crate) mod days;
pub(crate) mod draft;
pub(crate) mod draw;
pub(crate) mod fetch;
pub(crate) mod filing;
mod foot;
pub(crate) mod grant;
pub(crate) mod handoff;
pub(crate) mod html;
pub(crate) mod keys;
pub(crate) mod layout;
mod legend;
pub(crate) mod line;
pub(crate) mod links;
pub(crate) mod list;
pub(crate) mod mailwin;
pub(crate) mod picfetch;
mod picpaint;
pub(crate) mod pics;
pub(crate) mod plain;
pub(crate) mod query;
pub(crate) mod reader;
mod reading;
mod readmail;
pub(crate) mod rich;
pub(crate) mod row;
mod rowfit;
pub(crate) mod save;
pub(crate) mod setup;
pub(crate) mod signin;
pub(crate) mod smith;
mod states;
pub(crate) mod take;
pub(crate) mod triage;
pub(crate) mod unsub;
pub(crate) mod who;
pub(crate) mod wire;
pub(crate) mod worker;

use std::path::PathBuf;
use std::sync::mpsc::Receiver;

use grant::{Client, Grant};
pub(crate) use reading::Reading;
use wire::Thread;
use worker::{Ask, Worker};

pub(crate) enum Stage {
    /// No grant yet: how to make a client, and the one found, if any.
    Setup {
        found: Option<(PathBuf, Client)>,
    },
    SigningIn(Receiver<Result<Grant, String>>),
    Ready(Worker),
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
    /// `a`/`d`/`t` pressed before the thread's messages arrived.
    pub(crate) then: Option<handoff::Then>,
    /// A prompt for agent smith, for the app to deliver.
    pub(crate) to_smith: Option<handoff::ToSmith>,
    /// A reply smith is writing for a draft.
    pub(crate) waiting: Option<handoff::Waiting>,
    pub(crate) pics: pics::Pics,
    /// Why the list could not be had, while there are no rows to show.
    pub(crate) failed: Option<String>,
    /// When Gmail last answered a list or a check (ms since the epoch).
    pub(crate) checked_ms: Option<i64>,
    /// One line or two per row (`L`).
    pub(crate) density: list::Density,
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
        p.density = list::Density::picked();
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
            then: None,
            to_smith: None,
            waiting: None,
            pics: pics::Pics::default(),
            failed: None,
            checked_ms: None,
            density: list::Density::Auto,
            ticked_ms: crate::anim::now_ms(),
        }
    }

    /// Send `ask` to the worker; `false` when not signed in.
    pub(crate) fn ask(&mut self, ask: Ask) -> bool {
        let Stage::Ready(w) = &self.stage else {
            return false;
        };
        self.busy = !matches!(
            ask,
            Ask::Check | Ask::Picture { .. } | Ask::Save { .. } | Ask::Unsubscribe { .. }
        );
        w.tx.send(ask).is_ok()
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

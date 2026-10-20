//! What the nav's INBOX card shows (`navinboxcard`), and the worker that
//! keeps it: the unread count and the newest unread thread.
//!
//! Gmail charges each project's user 6,000 units a minute, so the card asks
//! once every five minutes (labels 1 + one unread thread 50), and not at all
//! while the inbox pane is open — the pane already asks each minute, and the
//! card shows what the pane holds. Signed out, the card is not there and
//! nothing is asked; whether a grant exists is re-read every 30 s.
use std::sync::mpsc::{self, Receiver};
use std::sync::{Mutex, RwLock};

use crate::inbox::wire::{sender_name, Thread};

const EVERY_MS: u64 = 5 * 60_000;
const GRANT_MS: u64 = 30_000;

#[derive(Clone, Debug, PartialEq, Default)]
pub(crate) enum Card {
    /// Not signed in: no card.
    #[default]
    Off,
    /// Signed in, the first count on its way.
    Checking,
    /// Unread threads, and the newest unread one's sender and subject.
    Count {
        unread: u64,
        newest: Option<(String, String)>,
    },
    /// What went wrong, in a few words.
    Problem(String),
}

type Peek = Result<(u64, Option<Thread>), String>;

#[derive(Default)]
struct Watch {
    signed: bool,
    grant_ms: Option<u64>,
    next_ms: u64,
    fetch: Option<Receiver<Peek>>,
}

static CARD: RwLock<Card> = RwLock::new(Card::Off);
static WATCH: Mutex<Option<Watch>> = Mutex::new(None);

pub(crate) fn state() -> Card {
    CARD.read().map(|c| c.clone()).unwrap_or_default()
}

/// Publish `next`; true when the card changed.
fn set(next: Card) -> bool {
    let mut c = CARD.write().unwrap_or_else(|p| p.into_inner());
    let changed = *c != next;
    *c = next;
    changed
}

/// The card for a count and the newest unread thread.
pub(crate) fn count(unread: u64, newest: Option<&Thread>) -> Card {
    let newest = newest.map(|t| (sender_name(&t.from), t.subject.trim().to_string()));
    Card::Count { unread, newest }
}

/// The card for what a fetch brought back. An ended grant says where to go.
pub(crate) fn landed(got: Peek) -> Card {
    match got {
        Ok((n, t)) => count(n, t.as_ref()),
        Err(e) if e.starts_with("invalid_grant") => {
            Card::Problem("signed out \u{2014} /inbox".into())
        }
        Err(e) => Card::Problem(crate::oauth::short_reason(&e)),
    }
}

impl crate::app::CrewApp {
    /// Once per tick: mirror the open pane, or ask Gmail when it is due.
    pub(crate) fn tick_inbox(&mut self, now_ms: u64) -> bool {
        let pane = self.panes.iter().find_map(|p| match &p.content {
            crate::pane::PaneContent::Inbox(i) => Some(i),
            _ => None,
        });
        let mut w = WATCH.lock().unwrap_or_else(|p| p.into_inner());
        let w = w.get_or_insert_with(Watch::default);
        if w.grant_ms
            .is_none_or(|at| now_ms.saturating_sub(at) >= GRANT_MS)
        {
            w.grant_ms = Some(now_ms);
            w.signed = crate::inbox::grant::load().is_some();
        }
        if !w.signed {
            w.fetch = None;
            return set(Card::Off);
        }
        if let Some(p) = pane.filter(|p| p.unread.is_some()) {
            let newest = p.rows.iter().find(|t| t.unread);
            return set(count(p.unread.unwrap_or(0), newest));
        }
        if let Some(got) = w.fetch.as_ref().and_then(|rx| rx.try_recv().ok()) {
            w.fetch = None;
            return set(landed(got));
        }
        if now_ms >= w.next_ms && w.fetch.is_none() {
            w.next_ms = now_ms + EVERY_MS;
            w.fetch = crate::inbox::grant::load().map(spawn);
            if state() == Card::Off {
                return set(Card::Checking);
            }
        }
        false
    }
}

impl crate::app::CrewApp {
    /// A click on the INBOX card (its rule or rows, not its gap) opens the
    /// inbox; the pane it opens or focuses.
    pub(crate) fn inbox_card_click(&mut self) -> Option<usize> {
        if self.config.nav_collapsed {
            return None;
        }
        let (rel_row, l) = self.sidebar_row()?;
        // +1: the card's border row sits above the slot's first content row.
        let first = l.inbox_top + 1;
        let on = l.inbox_rows > 0 && (first..first + l.inbox_rows - 1).contains(&rel_row);
        on.then(|| {
            self.inbox_command("");
            self.focused
        })
    }
}

fn spawn(grant: crate::inbox::grant::Grant) -> Receiver<Peek> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let got = crate::inbox::api::Gmail::new(grant).and_then(|mut g| g.peek());
        let _ = tx.send(got);
    });
    rx
}

#[cfg(test)]
#[path = "inboxglance_tests.rs"]
mod tests;

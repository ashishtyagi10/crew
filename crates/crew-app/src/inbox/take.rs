//! What the pane does with each thing that comes back: a finished sign-in,
//! and the worker's answers.
use super::grant::{self, Grant};
use super::setup;
use super::worker::{self, Ask, Got};
use super::{InboxPane, Stage};
use told::told;

#[path = "told.rs"]
mod told;

/// How often a signed-in pane asks whether the mailbox moved (1 unit).
const CHECK_MS: u64 = 60_000;
/// How often the setup page looks in Downloads for the client file.
const LOOK_MS: u64 = 2_000;

impl InboxPane {
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
                let landed = self.pics.drain();
                if landed {
                    self.sync_sizes();
                }
                let changed = landed || changed || self.picked();
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

    /// The browser sign-in ended: keep the grant and start listing, or say why
    /// not and go back to setup.
    pub(crate) fn signed(&mut self, got: Result<Grant, String>) -> bool {
        match got {
            Ok(g) => {
                self.note = match grant::save(&g) {
                    Ok(()) => "signed in".into(),
                    Err(e) => format!("signed in, but the grant was not saved: {e}"),
                };
                self.stage = Stage::Ready(worker::start(g));
                self.ask(Ask::List(self.query.clone()));
            }
            Err(e) => {
                self.note = format!("sign-in failed: {e}");
                // A second sign-in (`/inbox calendar`) that failed leaves the
                // first one as it was.
                self.stage = match grant::load() {
                    Some(g) => Stage::Ready(worker::start(g)),
                    None => Stage::Setup {
                        found: setup::look(),
                    },
                };
            }
        }
        true
    }

    pub(crate) fn take(&mut self, got: Got) {
        match got {
            Got::Account(email) => {
                if let Some(mut g) = grant::load().filter(|g| g.email != email) {
                    g.email.clone_from(&email);
                    let _ = grant::save(&g);
                }
                self.email = email;
            }
            Got::Page {
                query,
                rows,
                next,
                more,
                whole,
            } => {
                if query != self.query {
                    return; // an older search's page, overtaken
                }
                self.paged(rows, next.is_some(), more, whole);
            }
            Got::Unread(n) => self.unread = Some(n),
            got @ (Got::OnCalendar(..) | Got::Calendar(_) | Got::Answered(..)) => {
                self.take_cal(got)
            }
            Got::Checked => {
                self.checked_ms = Some(chrono::Utc::now().timestamp_millis());
                // Gmail answered: a failure the list still showed is over.
                self.failed = None;
            }
            Got::Opened(id, mails) => {
                self.busy = false;
                if let Some(r) = self.reading.as_mut().filter(|r| r.thread.id == id) {
                    // New mails (a reply just sent among them): a new layout.
                    *r.laid.get_mut().unwrap_or_else(|e| e.into_inner()) = None;
                    r.mails = Some(mails);
                    if let Some((then, _)) = self.then.take().filter(|(_, t)| *t == id) {
                        self.act(then);
                    }
                    self.fetch_carried();
                    self.fetch_web();
                    self.ask_calendar();
                }
                self.end_after_send();
            }
            Got::Unopened(id, e) => {
                self.busy = false;
                if let Some(r) = self.reading.as_mut().filter(|r| r.thread.id == id) {
                    r.failed = Some(e);
                }
                // `r`, `f` or `a` asked before it opened: nothing to act on.
                self.then = self.then.take().filter(|(_, t)| *t != id);
            }
            Got::Picture(key, bm) => self.pics.set(key, bm),
            Got::Saved(Ok(path)) => self.note = super::save::opened(&path),
            Got::Saved(Err(e)) => self.note = e,
            Got::Forwarded(to) => {
                self.compose = None;
                self.note = format!("forwarded to {}", super::addr::names(&to));
            }
            Got::Sent(to) => {
                self.compose = None;
                self.note = format!("sent to {}", super::addr::names(&to));
                // The thread again, the reply in it, shown (`end_after_send`).
                if let Some(r) = self.reading.as_mut() {
                    r.to_end = true;
                    let id = r.thread.id.clone();
                    self.ask(Ask::Open(id));
                }
            }
            Got::Unsubscribed(who) => {
                // `e` only over the mail it was from: answered after `n`, it
                // was offered over another thread, and `e` archived that one.
                let from = |m: &super::api::Mail| super::wire::sender_name(&m.from) == who;
                let mails = self.reading.as_ref().and_then(|r| r.mails.as_deref());
                let here = mails.is_some_and(|m| m.iter().any(|m| m.unsub.is_some() && from(m)));
                self.note = format!("unsubscribed from {who}");
                if here {
                    self.note.push_str(" \u{b7} e archives this mail");
                }
            }
            Got::Failed(e) if self.rows.is_empty() && self.reading.is_none() => {
                // Nothing listed: the list's own place says what went wrong.
                self.busy = false;
                self.failed = Some(e);
            }
            Got::NotSent(e) => {
                // A reply that did not go stays, words and all.
                if let Some(c) = self.compose.as_mut() {
                    c.sending = false;
                }
                self.note = format!("not sent: {e}");
            }
            Got::Failed(e) => {
                self.busy = false;
                self.note = told(e);
            }
            Got::Refused(id, e) => self.refused(&id, told(e)),
            Got::Ended(e) => self.signed_out(&e),
        }
    }
}

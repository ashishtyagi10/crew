//! The invite's keys in the reader, and what comes back for it (goal
//! 2026-10-22-inbox-calendar, phase 3). `c` adds it: to Google Calendar once
//! Google Calendar has answered for this invite, else through the system's
//! calendar app (`icsadd`). `y` opens the answer on the card, which names
//! who will be told; then `y` yes, `n` no, `m` maybe, or Esc for not now.
//! Nothing goes until that second key.
use winit::keyboard::{Key, NamedKey};

use super::reply::Answer;
use crate::inbox::grant::Client;
use crate::inbox::worker::{Ask, Got};
use crate::inbox::{signin, InboxPane, Stage};

impl InboxPane {
    /// The newest invite in the open thread, as it came: an update comes
    /// after the first.
    pub(crate) fn invite(&self) -> Option<String> {
        let mails = self.reading.as_ref()?.mails.as_ref()?;
        mails.iter().rev().find_map(|m| m.calendar.clone())
    }

    /// `c` or `y` in a thread; the line to say.
    pub(crate) fn invite_key(&mut self, ch: &str) -> String {
        // An answer to your own invite: never a REPLY sent to yourself.
        let inv = self.invite().and_then(|i| super::parse(&i));
        if let Some(said) = inv.and_then(|i| i.reply_line()) {
            return format!("{said} \u{b7} an answer, nothing to add or answer");
        }
        match ch {
            "c" => self.add_to_calendar(),
            _ => self.start_answer(),
        }
    }

    /// Whether Google Calendar has answered for `uid`: `Some(None)` when it
    /// is not there, `Some(Some(answer))` when it is.
    pub(crate) fn on_google(&self, uid: &str) -> Option<Option<String>> {
        let (u, said) = self.reading.as_ref()?.on_cal.clone()?;
        (u == uid).then_some(said)
    }

    fn start_answer(&mut self) -> String {
        let Some(inv) = self.invite().and_then(|i| super::parse(&i)) else {
            return "no invite in this thread".into();
        };
        if inv.cancelled {
            return "this meeting was cancelled \u{b7} nothing to answer".into();
        }
        let Some(org) = inv.organizer else {
            return "the invite names no organizer to answer".into();
        };
        self.with_reading(|r| r.answering = true);
        let who = if org.name.is_empty() {
            org.email
        } else {
            org.name
        };
        format!("{who} will be told")
    }

    pub(crate) fn answering(&self) -> bool {
        self.reading.as_ref().is_some_and(|r| r.answering)
    }

    /// A key while the answer is open: an answer goes, Esc closes it, any
    /// other key waits.
    pub(crate) fn answer_key(&mut self, key: &Key) {
        let a = match key {
            Key::Character(c) if c.as_str() == "y" => Some(Answer::Yes),
            Key::Character(c) if c.as_str() == "n" => Some(Answer::No),
            Key::Character(c) if c.as_str() == "m" => Some(Answer::Maybe),
            Key::Named(NamedKey::Escape) => None,
            _ => return,
        };
        self.with_reading(|r| r.answering = false);
        self.note = match (a, self.invite()) {
            (Some(answer), Some(ics)) => {
                let me = self.email.clone();
                self.ask(Ask::Answer { ics, answer, me });
                format!("telling them you {}\u{2026}", answer.said())
            }
            _ => String::new(),
        };
    }

    /// A thread just opened: ask Google Calendar about its invite.
    pub(crate) fn ask_calendar(&mut self) {
        if let Some(ics) = self.invite() {
            self.ask(Ask::OnCalendar(ics));
        }
    }

    /// What came back about an invite.
    pub(crate) fn take_cal(&mut self, got: Got) {
        match got {
            Got::OnCalendar(uid, said) => self.with_reading(|r| r.on_cal = Some((uid, said))),
            Got::Calendar(line) => {
                self.note = line;
                self.ask_calendar();
            }
            Got::Answered(uid, said, line) => {
                self.note = line;
                // The answer takes a second or two, and `e` moves on at once:
                // the thread open now may carry another invite, whose card
                // then said "you accepted" for one never answered.
                let here =
                    (self.invite().and_then(|i| super::parse(&i))).is_some_and(|i| i.uid == uid);
                self.with_reading(|r| {
                    if let Some((_, s)) = r.on_cal.as_mut().filter(|(u, _)| *u == uid) {
                        *s = Some(said.clone());
                    }
                    if here {
                        r.answered = Some(said);
                    }
                });
            }
            _ => {}
        }
    }

    /// `/inbox calendar`: sign in again, asking for Google Calendar's events
    /// beside Gmail's. The grant there is kept until the new one lands.
    pub(crate) fn calendar_sign_in(&mut self, client: Client) {
        match signin::spawn(client, true) {
            Some(rx) => {
                self.stage = Stage::SigningIn(rx);
                self.note = "turn the Calendar API on in the page that opened, then allow crew in the other one".into();
            }
            None => self.note = "could not open a port for the sign-in".into(),
        }
    }

    /// Change the open thread's reading, and lay it out again.
    fn with_reading(&mut self, f: impl FnOnce(&mut crate::inbox::Reading)) {
        if let Some(r) = self.reading.as_mut() {
            f(r);
            *r.laid.get_mut().unwrap_or_else(|e| e.into_inner()) = None;
        }
    }
}

#[cfg(test)]
#[path = "icsanswer_tests.rs"]
mod tests;

//! `a`, `r` and `t`: the thread to agent smith, a reply (or `f`), the
//! thread to `/todo`. Each acts on the open thread once its messages are
//! here; pressed in the list, it opens the thread and acts when it lands.
//!
//! What goes to smith leaves through [`ToSmith`]: the app owns both panes,
//! so it delivers the prompt and, for a draft, watches smith's answer
//! (`spawninbox::poll_inbox`) and hands the body back to [`InboxPane::drafted_by_smith`].
use super::compose::Kind;
use super::draft::{self, Answer};
use super::worker::Ask;
use super::{InboxPane, Reading};

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Then {
    Ask,
    /// Agent smith writes a reply: into the box when one is open.
    Draft,
    /// `r` and `f`: the reply box (`compose`).
    Reply,
    Forward,
    Todo,
}

/// A prompt for smith, the short line its transcript shows for it, and, for
/// a draft, what the draft will need besides the body.
pub(crate) struct ToSmith {
    pub prompt: String,
    pub shown: String,
    pub draft: Option<Answer>,
}

/// A reply smith is writing: the draft's headers, and the transcript length
/// when it was asked, so only its answer is read.
pub(crate) struct Waiting {
    pub answer: Answer,
    pub since: usize,
    pub at_ms: u64,
}

impl InboxPane {
    /// `a`, `r`, `f` or `t`: true when it was one of them.
    pub(crate) fn handoff(&mut self, ch: &str) -> bool {
        let then = match ch {
            "a" => Then::Ask,
            "r" => Then::Reply,
            "f" => Then::Forward,
            "t" => Then::Todo,
            _ => return false,
        };
        if self.reading.is_none() {
            let Some(thread) = self.rows.get(self.sel).cloned() else {
                return true;
            };
            self.ask(Ask::Open(thread.id.clone()));
            self.opened(&thread.id);
            self.reading = Some(Reading::new(thread));
            self.pics.clear();
        }
        match self.reading.as_ref().is_some_and(|r| r.mails.is_some()) {
            true => self.act(then),
            false => self.then = Some(then),
        }
        true
    }

    /// Do `then` with the open, loaded thread.
    pub(crate) fn act(&mut self, then: Then) {
        let Some(r) = self.reading.as_ref() else {
            return;
        };
        let subject = r.thread.subject.trim().to_string();
        match then {
            Then::Ask => {
                self.to_smith = Some(ToSmith {
                    prompt: super::smith::ask_prompt(r),
                    shown: format!("\u{2709} what does \u{201c}{subject}\u{201d} want from me?"),
                    draft: None,
                });
                self.note = "handed to agent smith".into();
            }
            Then::Draft => {
                let mails = r.mails.as_deref().unwrap_or_default();
                let answer = draft::answer_all(&r.thread.id, mails, &self.email);
                self.to_smith = Some(ToSmith {
                    prompt: super::smith::draft_prompt(r),
                    shown: format!("\u{2709} draft a reply to \u{201c}{subject}\u{201d}"),
                    draft: answer,
                });
                self.note =
                    "agent smith is writing a reply \u{2014} it lands as a Gmail draft, never sent"
                        .into();
            }
            Then::Reply => self.note = self.open_compose(Kind::Reply).into(),
            Then::Forward => self.note = self.open_compose(Kind::Forward).into(),
            Then::Todo => self.to_todo(),
        }
    }

    /// The reply smith wrote (or `None`: its answer held none) becomes a
    /// draft in the thread.
    pub(crate) fn drafted_by_smith(&mut self, body: Option<String>) {
        let Some(w) = self.waiting.take() else { return };
        match body {
            // Into the box, to be changed and sent by hand.
            Some(b) if self.compose.is_some() => {
                self.draft_in(&b);
                self.note = "agent smith drafted it \u{b7} change what you like, then send".into();
            }
            Some(b) => {
                let raw = draft::raw(&w.answer, &b);
                let ask = Ask::Draft {
                    thread_id: w.answer.thread_id.clone(),
                    raw,
                    subject: w.answer.subject,
                };
                self.ask(ask);
                self.note = "saving the draft\u{2026}".into();
            }
            None => {
                self.note =
                    "agent smith's answer held no draft \u{2014} press r to ask again".into()
            }
        }
    }

    fn to_todo(&mut self) {
        let Some(r) = self.reading.as_ref() else {
            return;
        };
        let title = super::smith::todo_title(r);
        let newest = r
            .mails
            .as_deref()
            .and_then(|m| m.last())
            .map_or("", |m| m.text.as_str());
        let due = super::smith::deadline(newest, crate::todopane::duedate::now_local());
        let now = crate::chattime::unix_now_ms();
        let item = crate::todopane::item::TodoItem {
            title: title.clone(),
            due_ms: due
                .as_ref()
                .and_then(|h| crate::todopane::duedate::to_epoch_ms(h.due)),
            due_has_time: due.as_ref().is_some_and(|h| h.has_time),
            created_ms: now,
            ..Default::default()
        };
        crate::todopane::store::mutate(|items| {
            let id = items.iter().map(|i| i.id).max().unwrap_or(0).max(now - 1) + 1;
            items.push(crate::todopane::item::TodoItem { id, ..item });
        });
        let when = match due {
            Some(h) => format!(" (due {})", h.due.format("%a %b %-d")),
            None => String::new(),
        };
        self.note = format!("added to /todo: {title}{when}");
    }
}

#[cfg(test)]
#[path = "handoff_tests.rs"]
mod tests;

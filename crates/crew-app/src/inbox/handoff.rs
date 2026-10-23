//! `a`, `r` and `t`: the thread to agent smith, a reply (or `f`), the
//! thread to `/todo`. Each acts on the open thread once its messages are
//! here; pressed in the list, it opens the thread and acts when it lands.
//!
//! What goes to smith leaves through [`ToSmith`]: the app owns both panes,
//! so it delivers the prompt and, for a draft, watches smith's answer
//! (`spawninbox::poll_inbox`) and hands the body back to [`InboxPane::drafted_by_smith`].
use super::compose::Kind;
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
    /// Tab on the todo card: agent smith fills it (`todofill`).
    Fill,
}

/// What the inbox waits on smith's answer for, if anything: a summary is
/// read in smith's pane; a draft goes into the reply box, a card's lines
/// into the todo card.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Wants {
    Read,
    Draft,
    Todo,
}

/// A prompt for smith, the short line its transcript shows for it, and
/// what the inbox waits for in its answer.
pub(crate) struct ToSmith {
    pub prompt: String,
    pub shown: String,
    pub wants: Wants,
    /// The thread asked about.
    pub thread: String,
}

/// A reply smith is writing for the box: the transcript length when it was
/// asked, so only its answer is read, and the thread it is about, so it
/// lands on no other.
pub(crate) struct Waiting {
    pub since: usize,
    pub at_ms: u64,
    pub wants: Wants,
    pub thread: String,
}

impl InboxPane {
    /// What agent smith is still busy with, when the inbox waits on it: the
    /// job it was asked for, not the key's. Tab on the card while a draft was
    /// coming said "still filling the card", and the other way about.
    pub(crate) fn smith_busy(&self) -> Option<String> {
        let wants =
            (self.waiting.as_ref().map(|w| w.wants)).or(self.to_smith.as_ref().map(|t| t.wants))?;
        let doing = match wants {
            Wants::Draft => "writing the draft",
            Wants::Todo => "filling the card",
            Wants::Read => "reading the thread",
        };
        Some(format!("agent smith is still {doing}\u{2026}"))
    }

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
        // Not here yet: kept for this thread's mails, and for no other's.
        match self
            .reading
            .as_ref()
            .map(|r| (r.mails.is_some(), r.thread.id.clone()))
        {
            Some((true, _)) => self.act(then),
            Some((false, id)) => self.then = Some((then, id)),
            None => {}
        }
        true
    }

    /// Do `then` with the open, loaded thread.
    pub(crate) fn act(&mut self, then: Then) {
        let Some(r) = self.reading.as_ref() else {
            return;
        };
        let subject = r.thread.subject.trim().to_string();
        let thread = r.thread.id.clone();
        match then {
            Then::Ask => {
                self.to_smith = Some(ToSmith {
                    prompt: super::smith::ask_prompt(r),
                    shown: format!("\u{2709} what does \u{201c}{subject}\u{201d} want from me?"),
                    wants: Wants::Read,
                    thread,
                });
                self.note = "handed to agent smith".into();
            }
            // Tab in the reply box: the words, written into it.
            Then::Draft => {
                self.to_smith = Some(ToSmith {
                    prompt: super::smith::draft_prompt(r),
                    shown: format!("\u{2709} draft a reply to \u{201c}{subject}\u{201d}"),
                    wants: Wants::Draft,
                    thread,
                });
                self.note = "agent smith is writing a draft into the box\u{2026}".into();
            }
            Then::Reply => self.note = self.open_compose(Kind::Reply).into(),
            Then::Forward => self.note = self.open_compose(Kind::Forward).into(),
            Then::Todo => self.note = self.open_todo(),
            Then::Fill => {
                let k = super::todocard::Known::here(&self.email);
                let projects: Vec<String> = k.projects.iter().chain(&k.folders).cloned().collect();
                self.to_smith = Some(ToSmith {
                    prompt: super::smith::todo_prompt(r, &projects, &self.email),
                    shown: format!("\u{2709} the todo in \u{201c}{subject}\u{201d}"),
                    wants: Wants::Todo,
                    thread,
                });
                self.note = "agent smith is filling the card\u{2026}".into();
            }
        }
    }

    /// The words smith wrote (or `None`: its answer held none) go into the
    /// reply box, to be changed and sent by hand. A box closed meanwhile
    /// (let go, or sent) lets them go too: nothing is saved behind your back.
    /// Smith's settled answer (`None`: its pane closed), to what the inbox
    /// was waiting for.
    pub(crate) fn from_smith(&mut self, answers: Option<Vec<String>>) {
        let texts = answers.unwrap_or_default();
        // Asked on one mail, answered with another open (Esc, then `t` on
        // the next): let go, never written onto that one's card or box.
        if let (Some(w), Some(r)) = (&self.waiting, &self.reading) {
            if w.thread != r.thread.id {
                self.waiting = None;
                self.note =
                    "agent smith\u{2019}s answer was about another mail \u{2014} let go".into();
                return;
            }
        }
        match self.waiting.as_ref().map(|w| w.wants) {
            Some(Wants::Todo) => {
                self.waiting = None;
                self.filled_by_smith(&texts);
            }
            _ => {
                self.drafted_by_smith(texts.iter().rev().find_map(|t| super::smith::draft_body(t)))
            }
        }
    }

    pub(crate) fn drafted_by_smith(&mut self, body: Option<String>) {
        if self.waiting.take().is_none() {
            return;
        }
        self.note = match (body, self.compose.is_some()) {
            (Some(b), true) => {
                self.draft_in(&b);
                "agent smith drafted it \u{b7} change what you like, then send".into()
            }
            (Some(_), false) => {
                "agent smith's draft came after the reply box closed \u{2014} let go".into()
            }
            (None, _) => {
                "agent smith's answer held no draft \u{2014} tab in the reply box asks again".into()
            }
        };
    }
}

#[cfg(test)]
#[path = "handoff_tests.rs"]
mod tests;

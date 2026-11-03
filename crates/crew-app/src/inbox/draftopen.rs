//! Drafts, on the pane's side (the user, 2026-11-03: "add a draft save so I
//! can finish it later"). ⌘S (Ctrl+S off a Mac) in the box saves it to
//! Gmail's Drafts and closes it; `D` in the list shows the drafts. A thread
//! opened with a draft in it puts the draft back in the box, as Gmail does:
//! under the thread when it answers one, under the list when it is a new
//! mail. Saved again it is the same draft; sent, it leaves Drafts.
use crate::inbox::compose::{Compose, Kind, On};
use crate::inbox::draft::Answer;
use crate::inbox::field::Field;
use crate::inbox::worker::Ask;
use crate::inbox::InboxPane;

/// The keycap that saves a draft: the chord the platform saves with.
pub(crate) const SAVE: &str = if cfg!(target_os = "macos") {
    "\u{2318}S"
} else {
    "ctrl+S"
};

impl InboxPane {
    /// ⌘S: the box to Gmail's Drafts. False when there is no box to save.
    pub(crate) fn save_draft(&mut self) -> bool {
        let ready = self.ready();
        let Some(c) = self.compose.as_mut().filter(|_| ready) else {
            return false;
        };
        if c.sending {
            self.note = "on its way \u{2014} the box closes when Gmail says so".into();
        } else if c.untouched() && c.draft.is_some() {
            self.compose = None;
            self.note = "the draft is as it was saved".into();
        } else if c.untouched() {
            self.note = "nothing to save yet".into();
        } else {
            c.sending = true;
            let o = c.outgoing();
            self.note = "saving the draft\u{2026}".into();
            self.ask(Ask::SaveDraft(Box::new(o)));
        }
        true
    }

    /// Gmail's answer to a save: the box goes, kept in Drafts; or stays.
    pub(crate) fn draft_saved(&mut self, saved: Result<String, String>) {
        match saved {
            Ok(_) => {
                self.compose = None;
                self.note = "saved to Gmail drafts \u{b7} D in the list shows them".into();
            }
            Err(e) => {
                if let Some(c) = self.compose.as_mut() {
                    c.sending = false;
                }
                self.note = format!("draft not saved: {e}");
            }
        }
    }

    /// The open thread's newest draft, out of its mails and into the box.
    pub(crate) fn reopen_draft(&mut self) {
        if self.compose.is_some() || self.todo.is_some() {
            return;
        }
        let Some(r) = self.reading.as_mut() else {
            return;
        };
        let Some(mails) = r.mails.as_mut() else {
            return;
        };
        let found =
            |m: &crate::inbox::api::Mail| m.draft.as_ref().is_some_and(|d| !d.id.is_empty());
        let Some(i) = mails.iter().rposition(found) else {
            return;
        };
        let m = mails.remove(i);
        let draft = m.draft.as_deref().cloned().unwrap_or_default();
        let theirs = !mails.is_empty();
        let refs = m.reply.references.trim();
        let answer = Answer {
            thread_id: if theirs {
                r.thread.id.clone()
            } else {
                String::new()
            },
            to: m.reply.to.clone(),
            cc: m.reply.cc.clone(),
            subject: m.reply.subject.clone(),
            in_reply_to: refs.split_whitespace().last().unwrap_or_default().into(),
            references: refs.into(),
        };
        let kind = match theirs {
            true => Kind::Reply,
            // A new mail: nothing to read above it, so under the list.
            false => {
                self.reading = None;
                Kind::New
            }
        };
        let mut c = Compose::of(kind, answer, None);
        let words = m.text.trim_end().to_string();
        c.body = Field::with(&words);
        c.base = words;
        c.files = draft.files;
        c.draft = Some(draft.id);
        c.on = On::Body;
        self.compose = Some(c);
        self.note = format!(
            "your draft \u{b7} {SAVE} saves it, {} sends it",
            crate::inbox::footkeys::SEND
        );
    }
}

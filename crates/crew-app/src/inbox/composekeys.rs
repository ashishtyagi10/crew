//! The reply box's keys: while it is open every key is its own. Tab and
//! Enter on the `to` or `cc` line go on to the next; the arrows move
//! up and down through the lines, the words and the row of files; in the
//! words Tab asks agent smith to write a draft there, which is yours to
//! change before it goes. On the row of files ← and → choose, Enter opens
//! the file (or the chooser, on `+ attach`) and Backspace takes it off.
use winit::keyboard::{Key, NamedKey};

use super::compose::{Kind, On};
use super::InboxPane;

impl InboxPane {
    /// A key while the box is open.
    pub(crate) fn compose_key(&mut self, key: &Key) {
        let Some(c) = self.compose.as_mut() else {
            return;
        };
        let armed = std::mem::take(&mut c.armed);
        let named = |n: NamedKey| matches!(key, Key::Named(k) if *k == n);
        let (on, forward) = (c.on, c.kind == Kind::Forward);
        let next = match on {
            On::To => On::Cc,
            On::Cc => On::Body,
            On::Body => On::Files,
            On::Files => On::To,
        };
        match key {
            // On its way: the box goes when Gmail says it went, so that word
            // cannot close a box opened since.
            Key::Named(NamedKey::Escape) if c.sending => {
                self.note = "on its way \u{2014} the box closes when Gmail says it went".into();
            }
            Key::Named(NamedKey::Escape) if armed || c.untouched() => {
                self.compose = None;
                self.note = String::new();
            }
            Key::Named(NamedKey::Escape) => {
                c.armed = true;
                self.note = "esc again discards it".into();
            }
            // Once: a second Tab while smith writes would land a second draft.
            Key::Named(NamedKey::Tab) if on == On::Body && !forward => match self.smith_busy() {
                Some(line) => self.note = line,
                None => self.act(super::handoff::Then::Draft),
            },
            Key::Named(NamedKey::Tab) => c.on = next,
            Key::Named(NamedKey::Enter) if matches!(on, On::To | On::Cc) => c.on = next,
            _ if on == On::Files => self.files_key(key),
            // A line up or down in the words, else the place above or below.
            Key::Named(NamedKey::ArrowUp) => {
                if !c.field().line(true) {
                    c.on = match on {
                        On::Body => On::Cc,
                        _ => On::To,
                    };
                }
            }
            Key::Named(NamedKey::ArrowDown) => {
                if !c.field().line(false) {
                    c.on = next;
                }
            }
            _ if named(NamedKey::Backspace) => c.field().back(),
            _ if named(NamedKey::Delete) => c.field().delete(),
            _ if named(NamedKey::ArrowLeft) => c.field().left(),
            _ if named(NamedKey::ArrowRight) => c.field().right(),
            _ if named(NamedKey::Home) => c.field().home(),
            _ if named(NamedKey::End) => c.field().end(),
            Key::Named(NamedKey::Enter) => self.type_in("\n"),
            Key::Named(NamedKey::Space) => self.type_in(" "),
            Key::Character(s) => self.type_in(s),
            _ => {}
        }
    }

    /// A key on the row of files: choose, open, take off, or attach.
    fn files_key(&mut self, key: &Key) {
        let Some(c) = self.compose.as_mut() else {
            return;
        };
        let (pick, n) = (c.pick.min(c.files.len()), c.files.len());
        match key {
            Key::Named(NamedKey::ArrowLeft) => c.pick = pick.saturating_sub(1),
            Key::Named(NamedKey::ArrowRight) => c.pick = (pick + 1).min(n),
            Key::Named(NamedKey::ArrowUp) => c.on = On::Body,
            Key::Named(NamedKey::Enter | NamedKey::Space) if pick == n => self.choose_files(),
            Key::Named(NamedKey::Enter | NamedKey::Space) => self.open_attached(pick),
            Key::Named(NamedKey::Backspace | NamedKey::Delete) => self.unattach(pick),
            _ => {}
        }
    }

    /// `text` at the caret, on the line or in the words: a key, a paste.
    pub(crate) fn type_in(&mut self, text: &str) {
        if let Some(t) = self.todo.as_mut() {
            return t.field().put(text, true);
        }
        let Some(c) = self.compose.as_mut().filter(|c| !c.sending) else {
            return;
        };
        match c.on {
            On::Files => {}
            On::Body => c.body.put(text, false),
            _ => c.field().put(text, true),
        }
    }

    /// agent smith's draft, into the words wherever the caret was.
    pub(crate) fn draft_in(&mut self, text: &str) {
        let Some(c) = self.compose.as_mut().filter(|c| !c.sending) else {
            return;
        };
        c.on = On::Body;
        c.body.put(text, false);
    }

    /// The keys the foot shows while the box is open: what works where the
    /// caret is.
    pub(crate) fn compose_keys(&self) -> super::footkeys::Keys {
        use super::footkeys::{ADDRESS, COMPOSE, FILES, FORWARD};
        match self.compose.as_ref().map(|c| (c.on, c.kind)) {
            Some((On::To | On::Cc, _)) => ADDRESS,
            Some((On::Files, _)) => FILES,
            Some((On::Body, Kind::Forward)) => FORWARD,
            _ => COMPOSE,
        }
    }
}

#[cfg(test)]
#[path = "composekeys_tests.rs"]
pub(crate) mod tests;

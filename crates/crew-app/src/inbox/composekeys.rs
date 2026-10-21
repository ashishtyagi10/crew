//! The reply box's keys: while it is open every key is its own. Tab and
//! Enter on the `to` or `cc` line go on to the next; the arrows move up and
//! down through the lines and the words; in the words Tab asks agent smith
//! to write a draft there, which is yours to change before it goes.
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
            On::Body => On::To,
        };
        match key {
            Key::Named(NamedKey::Escape) if armed || c.untouched() => {
                self.compose = None;
                self.note = String::new();
            }
            Key::Named(NamedKey::Escape) => {
                c.armed = true;
                self.note = "esc again discards it".into();
            }
            Key::Named(NamedKey::Tab) if on == On::Body && !forward => {
                self.act(super::handoff::Then::Draft);
                self.note = "agent smith is writing a draft into the box\u{2026}".into();
            }
            Key::Named(NamedKey::Tab) => c.on = next,
            Key::Named(NamedKey::Enter) if on != On::Body => c.on = next,
            // A line up or down in the words, else the line above or below.
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
                    c.on = match on {
                        On::To => On::Cc,
                        _ => On::Body,
                    };
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

    /// `text` at the caret, on the line or in the words: a key, a paste.
    pub(crate) fn type_in(&mut self, text: &str) {
        let Some(c) = self.compose.as_mut().filter(|c| !c.sending) else {
            return;
        };
        let line = c.on != On::Body;
        c.field().put(text, line);
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
        use super::footkeys::{ADDRESS, COMPOSE, FORWARD};
        match self.compose.as_ref().map(|c| (c.on, c.kind)) {
            Some((On::To | On::Cc, _)) => ADDRESS,
            Some((On::Body, Kind::Forward)) => FORWARD,
            _ => COMPOSE,
        }
    }

    /// A click on row `row` of a pane `cols` by `rows`: the caret to the box's
    /// line there, or its words. True when it was on the box.
    pub(crate) fn compose_click(&mut self, cols: u16, rows: u16, row: u16) -> bool {
        let h = self.compose_h(cols, rows);
        let top = self.body_rows(cols, rows).saturating_sub(h);
        let Some(c) = self.compose.as_mut() else {
            return false;
        };
        if h == 0 || row < top || row >= top + h {
            return false;
        }
        c.on = match row - top {
            0 | 1 => On::To,
            2 => On::Cc,
            _ => On::Body,
        };
        true
    }
}

#[cfg(test)]
#[path = "composekeys_tests.rs"]
pub(crate) mod tests;

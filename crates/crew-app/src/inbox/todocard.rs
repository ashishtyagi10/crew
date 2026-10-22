//! `t` on a thread: a card under the open mail, as `r` opens the reply box
//! (goal 2026-10-22-inbox-smart-todo). It holds the `/todo` row the mail
//! asks for, filled in by crew: the title, `@project`, `#assignee` and due,
//! with why beside each guess. Up and Down move between the lines, typing
//! changes one, Enter adds the row, and Esc lets it go (a second Esc once
//! something changed). Nothing lands before Enter: a project guessed from a
//! stranger's mail could send an agent into the wrong checkout
//! (`projectdir`), so a guess is seen before it is saved.
use winit::keyboard::{Key, NamedKey};

use super::field::Field;
use super::InboxPane;

#[path = "todoadd.rs"]
mod add;
#[path = "todocarddraw.rs"]
mod draw;
#[path = "todoguess.rs"]
mod guess;

pub(crate) use add::due_of;

/// The card's lines, top to bottom.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Line {
    Title,
    Project,
    Who,
    Due,
}

pub(crate) const LINES: [Line; 4] = [Line::Title, Line::Project, Line::Who, Line::Due];

/// What crew writes on a line, and why ("" when nothing said so).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Guess {
    pub text: String,
    pub why: String,
}

pub(crate) struct TodoCard {
    pub lines: [Field; 4],
    pub why: [String; 4],
    /// The words each line opened with: a changed line drops its why, and
    /// Esc lets an unchanged card go at once.
    opened: [String; 4],
    pub on: Line,
    /// Esc pressed once over a change: a second lets it go.
    pub armed: bool,
}

impl TodoCard {
    pub(crate) fn new(guess: [Guess; 4]) -> Self {
        Self {
            lines: guess.clone().map(|g| Field::with(&g.text)),
            why: guess.clone().map(|g| g.why),
            opened: guess.map(|g| g.text),
            on: Line::Title,
            armed: false,
        }
    }

    pub(crate) fn text(&self, l: Line) -> &str {
        self.lines[l as usize].text.trim()
    }

    pub(crate) fn field(&mut self) -> &mut Field {
        &mut self.lines[self.on as usize]
    }

    /// Line `l` as crew wrote it: its why still holds.
    pub(crate) fn kept(&self, l: Line) -> bool {
        self.lines[l as usize].text == self.opened[l as usize]
    }

    fn untouched(&self) -> bool {
        LINES.iter().all(|&l| self.kept(l))
    }

    fn step(&mut self, down: bool) {
        let i = self.on as usize;
        self.on = LINES[if down {
            (i + 1).min(3)
        } else {
            i.saturating_sub(1)
        }];
    }
}

impl InboxPane {
    /// `t` on the open, loaded thread: the card, filled in. The line to say.
    pub(crate) fn open_todo(&mut self) -> String {
        if self.compose.is_some() {
            return super::advance::HELD.into();
        }
        let Some(r) = self.reading.as_ref() else {
            return String::new();
        };
        let known = guess::Known::here(&self.email);
        self.todo = Some(TodoCard::new(guess::guess(r, &known)));
        String::new()
    }

    /// Whether a card (the reply box or the todo) is open under the mail.
    pub(crate) fn carded(&self) -> bool {
        self.compose.is_some() || self.todo.is_some()
    }

    /// A key while a card is open: the todo's here, the box's in `composekeys`.
    pub(crate) fn card_key(&mut self, key: &Key) {
        if self.todo.is_none() {
            return self.compose_key(key);
        }
        let Some(c) = self.todo.as_mut() else {
            return;
        };
        let armed = std::mem::take(&mut c.armed);
        match key {
            Key::Named(NamedKey::Escape) if armed || c.untouched() => {
                self.todo = None;
                self.note.clear();
            }
            Key::Named(NamedKey::Escape) => {
                c.armed = true;
                self.note = "esc again discards it".into();
            }
            Key::Named(NamedKey::Enter) => self.add_todo(),
            Key::Named(NamedKey::ArrowUp) => c.step(false),
            Key::Named(NamedKey::ArrowDown) => c.step(true),
            Key::Named(NamedKey::Backspace) => c.field().back(),
            Key::Named(NamedKey::Delete) => c.field().delete(),
            Key::Named(NamedKey::ArrowLeft) => c.field().left(),
            Key::Named(NamedKey::ArrowRight) => c.field().right(),
            Key::Named(NamedKey::Home) => c.field().home(),
            Key::Named(NamedKey::End) => c.field().end(),
            Key::Named(NamedKey::Space) => c.field().put(" ", true),
            Key::Character(s) => c.field().put(s, true),
            _ => {}
        }
    }
}

#[cfg(test)]
#[path = "todocard_tests.rs"]
mod tests;

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
pub(crate) use guess::Known;

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

    /// Line `i` as smith filled it: the words, why, and kept as written.
    fn set(&mut self, i: usize, g: Guess) {
        self.lines[i] = Field::with(&g.text);
        self.opened[i].clone_from(&g.text);
        self.why[i] = g.why;
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
        self.compose.is_some() || self.todo.is_some() || self.answering()
    }

    /// A key while a card is open: the todo's here, the box's in `composekeys`,
    /// an invite's answer in `ics::answer`.
    pub(crate) fn card_key(&mut self, key: &Key) {
        if self.answering() {
            return self.answer_key(key);
        }
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
            // Once: a second Tab while smith reads would fill it twice.
            Key::Named(NamedKey::Tab) => match self.smith_busy() {
                Some(line) => self.note = line,
                None => self.act(super::handoff::Then::Fill),
            },
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

impl InboxPane {
    /// Smith's answer for the card: each line it filled that crew accepts
    /// (`todofill`) takes the place of a line still as crew wrote it; a line
    /// you changed stays yours.
    pub(crate) fn filled_by_smith(&mut self, texts: &[String]) {
        let (Some(c), Some(r)) = (self.todo.as_mut(), self.reading.as_ref()) else {
            self.note =
                "agent smith\u{2019}s todo came after the card closed \u{2014} let go".into();
            return;
        };
        let k = Known::here(&self.email);
        let Some(lines) = texts.iter().rev().find_map(|t| guess::check(t, r, &k)) else {
            self.note = "agent smith\u{2019}s answer held no card \u{2014} tab asks again".into();
            return;
        };
        let mut n = 0;
        for (i, g) in lines.into_iter().enumerate() {
            if let Some(g) = g.filter(|_| c.kept(LINES[i])) {
                c.set(i, g);
                n += 1;
            }
        }
        self.note = match n {
            0 => "agent smith left the card as it was".into(),
            n => format!(
                "agent smith filled {n} of its lines \u{b7} change what you like, then enter"
            ),
        };
    }
}

#[cfg(test)]
#[path = "todocard_tests.rs"]
mod tests;

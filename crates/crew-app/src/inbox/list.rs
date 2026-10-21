//! The thread list drawn: as many rows as fit above the foot, the selected
//! one in view, two lines each when the pane holds at least six of them with
//! air between, and one line each when it does not. `L` picks one or the
//! other, and the pick holds for the rest of the session, in every inbox
//! pane opened. Each day's name is a quiet word over its threads: no rules.
use std::sync::atomic::{AtomicU8, Ordering};

use crew_render::Paint;

use super::draw::Canvas;
use super::layout::{Shown, Slot};
use super::row::{self, Ctx};
use super::wire::Thread;
use super::InboxPane;

/// One line or two per row: chosen by the room (`Auto`), or by `L`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) enum Density {
    #[default]
    Auto,
    One,
    Two,
}

/// The session's `L` pick: what a pane opened later starts with.
static PICKED: AtomicU8 = AtomicU8::new(0);

/// Two-line rows are worth it only when this many of them fit, each with
/// the blank row under it.
const TWO_LINE_ROWS: u16 = 6;

impl Density {
    /// The session's pick, for a pane being opened.
    pub(crate) fn picked() -> Self {
        match PICKED.load(Ordering::Relaxed) {
            1 => Self::One,
            2 => Self::Two,
            _ => Self::Auto,
        }
    }

    /// Two lines per row in the `rows` above the foot?
    pub(crate) fn two(self, rows: u16) -> bool {
        match self {
            Self::One => false,
            Self::Two => true,
            // Less the row of air at the top and the one over the foot.
            Self::Auto => rows.saturating_sub(2) / 3 >= TWO_LINE_ROWS,
        }
    }
}

impl InboxPane {
    /// `L`: the other density, kept for the session. The line to say.
    pub(crate) fn flip_density(&mut self, rows: u16) -> &'static str {
        let two = !self.density.two(rows);
        self.density = if two { Density::Two } else { Density::One };
        PICKED.store(if two { 2 } else { 1 }, Ordering::Relaxed);
        match two {
            true => "two lines a thread \u{b7} L for one",
            false => "one line a thread \u{b7} L for two",
        }
    }

    fn ctx(&self, cols: u16, shown: &Shown) -> Ctx<'_> {
        let on: Vec<Thread> = shown.threads().map(|(_, i)| self.rows[i].clone()).collect();
        Ctx {
            cols,
            two: shown.lines == 2,
            me: &self.email,
            now: chrono::Utc::now().timestamp_millis(),
            who_w: row::who_w(&on, cols, &self.email),
        }
    }

    pub(crate) fn list(&self, cv: &mut Canvas) {
        if self.rows.is_empty() {
            return self.empty(cv);
        }
        let shown = self.shown(cv.rows);
        let x = self.ctx(cv.cols, &shown);
        let muted = crew_theme::theme().text_muted;
        for (r, slot) in &shown.slots {
            match slot {
                Slot::Band(day) => cv.text(*r, row::TEXT, cv.cols, day, muted, false),
                Slot::Thread(i) => row::cells(&mut cv.out, &self.rows[*i], *r, *i == self.sel, &x),
            }
        }
    }

    /// What the list paints: nothing over rows (the air divides them), and
    /// the empty list's outline rows or mark.
    pub(crate) fn list_paint(&self, cols: u16, rows: u16, aspect: f32) -> Vec<Paint> {
        match self.rows.is_empty() {
            true => self.empty_paint(cols, rows, aspect),
            false => Vec::new(),
        }
    }
}

#[cfg(test)]
#[path = "list_tests.rs"]
mod tests;

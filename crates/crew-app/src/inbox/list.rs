//! The thread list drawn: as many rows as fit above the foot, the selected
//! one in view, two lines each when the pane holds at least eight of them
//! and one line each when it does not. `L` picks one or the other, and the
//! pick holds for the rest of the session, in every inbox pane opened.
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

/// Two-line rows are worth it only when this many of them fit.
const TWO_LINE_ROWS: u16 = 8;

impl Density {
    /// The session's pick, for a pane being opened.
    pub(crate) fn picked() -> Self {
        match PICKED.load(Ordering::Relaxed) {
            1 => Self::One,
            2 => Self::Two,
            _ => Self::Auto,
        }
    }

    /// Two lines per row in a pane `rows` tall?
    pub(crate) fn two(self, rows: u16) -> bool {
        match self {
            Self::One => false,
            Self::Two => true,
            Self::Auto => rows.saturating_sub(1) / 2 >= TWO_LINE_ROWS,
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
                Slot::Band(day) => cv.text(*r, 1, cv.cols, day, muted, true),
                Slot::Thread(i) => row::cells(&mut cv.out, &self.rows[*i], *r, *i == self.sel, &x),
            }
        }
    }

    /// What the list paints: the selected thread's accent bar, a rule out
    /// from each day's name, and on two-line rows a fainter one between
    /// threads of the same day.
    pub(crate) fn list_paint(&self, cols: u16, rows: u16, aspect: f32) -> Vec<Paint> {
        if self.rows.is_empty() {
            return self.empty_paint(cols, rows, aspect);
        }
        let shown = self.shown(rows);
        let (lines, end) = (shown.lines, f32::from(cols.saturating_sub(1)));
        let muted = crew_theme::theme().text_muted;
        let rule = |x: f32, y: f32, alpha: f32| Paint {
            alpha,
            ..Paint::solid(x, y, (end - x).max(0.0), 0.06, muted)
        };
        let mut paint = Vec::new();
        let mut prev: Option<usize> = None;
        for (r, slot) in &shown.slots {
            let y = f32::from(*r);
            match slot {
                Slot::Band(day) => {
                    let x = 2.0 + crate::chatwidth::str_w(day) as f32;
                    paint.push(rule(x, y + 0.5, 0.3));
                    prev = None;
                }
                Slot::Thread(i) => {
                    // Not against the wash: it has its own edge.
                    let washed = *i == self.sel || prev == Some(self.sel);
                    if lines == 2 && prev.is_some() && !washed {
                        paint.push(rule(row::NAMES as f32, y + 0.1, 0.2));
                    }
                    if *i == self.sel {
                        paint.push(row::bar(*r, lines));
                    }
                    prev = Some(*i);
                }
            }
        }
        paint
    }
}

#[cfg(test)]
#[path = "list_tests.rs"]
mod tests;

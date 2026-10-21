//! The thread list drawn: as many rows as fit above the foot, the selected
//! one in view, two lines each when the pane holds at least eight of them
//! and one line each when it does not. `L` picks one or the other, and the
//! pick holds for the rest of the session, in every inbox pane opened.
use std::sync::atomic::{AtomicU8, Ordering};

use crew_render::{CellView, Paint};

use super::draw::Canvas;
use super::row::{self, Ctx};
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

    /// The rows on screen: the first thread shown, the lines each takes, and
    /// how many are shown.
    pub(crate) fn shown(&self, rows: u16) -> (usize, u16, usize) {
        let lines = if self.density.two(rows) { 2 } else { 1 };
        let fit = usize::from((rows.saturating_sub(1) / lines).max(1));
        let top = self.sel.saturating_sub(fit - 1);
        (top, lines, fit.min(self.rows.len().saturating_sub(top)))
    }

    fn ctx(&self, cols: u16, rows: u16) -> Ctx<'_> {
        let (top, lines, n) = self.shown(rows);
        Ctx {
            cols,
            two: lines == 2,
            me: &self.email,
            now: chrono::Utc::now().timestamp_millis(),
            who_w: row::who_w(&self.rows[top..top + n], cols, &self.email),
        }
    }

    pub(crate) fn list(&self, cv: &mut Canvas) {
        let t = crew_theme::theme();
        if self.rows.is_empty() {
            let said = match (self.busy, self.query.trim()) {
                (true, _) => "loading\u{2026}".to_string(),
                (false, "") => "nothing in the inbox".to_string(),
                (false, q) => format!("no mail matches \u{201c}{q}\u{201d}"),
            };
            cv.text(1, 2, cv.cols, &said, t.text_muted, false);
            return;
        }
        let (top, lines, n) = self.shown(cv.rows);
        let x = self.ctx(cv.cols, cv.rows);
        let mut out: Vec<CellView> = Vec::new();
        for (k, th) in self.rows[top..top + n].iter().enumerate() {
            let at = k as u16 * lines;
            row::cells(&mut out, th, at, top + k == self.sel, &x);
        }
        cv.out.extend(out);
    }

    /// What the list paints: the selected row's accent bar, and on two-line
    /// rows a hairline between threads.
    pub(crate) fn list_paint(&self, cols: u16, rows: u16) -> Vec<Paint> {
        if self.rows.is_empty() {
            return Vec::new();
        }
        let (top, lines, n) = self.shown(rows);
        let mut paint = Vec::new();
        if lines == 2 {
            let rule = crew_theme::theme().text_muted;
            // Not against the wash: it has its own edge.
            let washed = |k: usize| top + k == self.sel || top + k == self.sel + 1;
            for k in (1..n).filter(|&k| !washed(k)) {
                let y = (k as u16 * lines) as f32;
                let mut p = Paint::solid(row::NAMES as f32, y + 0.1, 0.0, 0.06, rule);
                p.w = f32::from(cols.saturating_sub(1)) - p.x;
                p.alpha = 0.25;
                paint.push(p);
            }
        }
        if let Some(k) = self.sel.checked_sub(top).filter(|&k| k < n) {
            paint.push(row::bar(k as u16 * lines, lines));
        }
        paint
    }
}

#[cfg(test)]
#[path = "list_tests.rs"]
mod tests;

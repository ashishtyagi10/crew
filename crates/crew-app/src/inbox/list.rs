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

    pub(super) fn ctx(&self, cols: u16, shown: &Shown) -> Ctx<'_> {
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
        let now = crate::anim::now_ms();
        let shown = self.shown_at(cv.rows, now);
        let x = self.ctx(cv.cols, &shown);
        let t = crew_theme::theme();
        let chosen = crew_theme::readable::selection_bg(t);
        // While the selection glides its wash is paint (`motion_paint`), and
        // the row it is going to waits for it.
        let gliding = self.motion.glide(now).is_some();
        for (r, slot) in &shown.slots {
            match slot {
                Slot::Band(day) => cv.text(*r, row::TEXT, cv.cols, day, t.text_muted, false),
                Slot::Thread(i) => {
                    let wash = match (*i == self.sel, self.hover == Some(*i)) {
                        (true, _) => (!gliding).then_some(chosen),
                        (false, true) => Some(crate::anim::lerp_rgb(t.page_bg, chosen, 0.5)),
                        (false, false) => None,
                    };
                    row::washed(&mut cv.out, &self.rows[*i], *r, wash, &x);
                }
                Slot::Fold(left) => {
                    let Some((_, th, _, fade)) = self.motion.folding(shown.lines, now) else {
                        continue;
                    };
                    let mut gone = Vec::new();
                    row::washed(&mut gone, th, *r, None, &x);
                    gone.retain(|c| c.row < r + left);
                    for c in &mut gone {
                        c.fg = crate::anim::lerp_rgb(c.fg, t.page_bg, fade);
                    }
                    cv.out.extend(gone);
                }
            }
        }
    }

    /// What the list paints: nothing over rows (the air divides them) but
    /// its motion, and the empty list's outline rows or mark.
    pub(crate) fn list_paint(&self, cols: u16, rows: u16, aspect: f32) -> Vec<Paint> {
        match self.rows.is_empty() {
            true => self.empty_paint(cols, rows, aspect),
            false => self.motion_paint(cols, rows),
        }
    }

    /// New mail's glow, and the selection's wash on its way to its row.
    fn motion_paint(&self, cols: u16, rows: u16) -> Vec<Paint> {
        let now = crate::anim::now_ms();
        let shown = self.shown_at(rows, now);
        let (w, lines) = (f32::from(cols), f32::from(shown.lines));
        let at = |i: usize| {
            shown
                .threads()
                .find(|&(_, j)| j == i)
                .map(|(r, _)| f32::from(r))
        };
        let t = crew_theme::theme();
        let mut paint = Vec::new();
        for (r, i) in shown.threads() {
            if let Some(g) = self.motion.glow(&self.rows[i].id, now) {
                let glow = Paint::solid(0.0, f32::from(r), w, lines, crate::palette::accent());
                paint.push(Paint {
                    alpha: 0.22 * g,
                    ..glow
                });
            }
        }
        if let (Some((from, k)), Some(to)) = (self.motion.glide(now), at(self.sel)) {
            let y = at(from).map_or(to, |from| from + (to - from) * k);
            let chosen = crew_theme::readable::selection_bg(t);
            paint.push(Paint::solid(0.0, y, w, lines, chosen));
        }
        paint
    }
}

#[cfg(test)]
#[path = "list_tests.rs"]
mod tests;

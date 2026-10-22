//! The todo card drawn (`todocard`): `todo` on its top edge as every crew
//! card wears its name, and its four lines, each a muted name, the words,
//! and beside a guess crew made, why. A due line being typed says what it
//! reads as; an empty line says what it means.
use super::super::cards::{frame, Rect};
use super::super::composedraw::{text_w, X};
use super::super::draw::Canvas;
use super::super::InboxPane;
use super::{due_of, Line, LINES};

/// Where a line's words start, after its name (`assignee`, the longest).
const VALUE: u16 = X + 10;
/// Its four lines and its two edges.
const H: u16 = 6;

impl InboxPane {
    /// The rows the todo card takes, its edges included: none when it is
    /// closed, and never as tall as the mail's card above it.
    pub(crate) fn todo_h(&self, cols: u16, rows: u16) -> u16 {
        match self.todo {
            Some(_) => H.min(self.body_rows(cols, rows).saturating_sub(1) / 2),
            None => 0,
        }
    }

    /// The first line shown: a card too short for all four keeps the line
    /// being written in view.
    pub(crate) fn todo_first(&self, h: u16) -> usize {
        let room = usize::from(h.saturating_sub(2)).max(1);
        self.todo
            .as_ref()
            .map_or(0, |c| (c.on as usize + 1).saturating_sub(room))
    }

    /// A click on the pane's `(row, col)`: on the card, the caret to the end
    /// of the line there. True when it was on the card.
    pub(crate) fn todo_click(&mut self, cols: u16, rows: u16, row: u16, col: u16) -> bool {
        let Some(b) = self.cards(cols, rows).reply else {
            return false;
        };
        let (Some((r, _)), first) = (b.local(row, col), self.todo_first(b.h)) else {
            return false;
        };
        let Some(c) = self.todo.as_mut() else {
            return false;
        };
        let at = r.checked_sub(1).filter(|_| r + 1 < b.h);
        if let Some(&line) = at.and_then(|i| LINES.get(first + usize::from(i))) {
            c.on = line;
            let f = c.field();
            f.at = f.text.chars().count();
        }
        true
    }

    /// The card on `cv`, a canvas of its own a column wider than the card
    /// (`w` wide), as the reply box draws.
    pub(crate) fn draw_todo(&self, cv: &mut Canvas, w: u16) {
        let Some(c) = &self.todo else {
            return;
        };
        let t = crew_theme::theme();
        let h = cv.rows;
        frame(
            cv,
            Rect {
                col: 0,
                row: 0,
                w,
                h,
            },
            "todo",
            "",
            true,
        );
        let end = X + text_w(w) as u16;
        let first = self.todo_first(h);
        for (i, &line) in LINES.iter().enumerate().skip(first) {
            let row = 1 + (i - first) as u16;
            if row + 1 >= h {
                break;
            }
            let focused = c.on == line;
            let (name, mark, empty) = match line {
                Line::Title => ("title", "", "what to do"),
                Line::Project => ("project", "@", "none"),
                Line::Who => ("assignee", "#", "you"),
                Line::Due => ("due", "", "none \u{b7} fri, oct 24, in 2 days"),
            };
            let label = if focused { t.ink } else { t.text_muted };
            cv.text(row, X, VALUE, name, label, focused);
            let f = &c.lines[i];
            let room = usize::from(end.saturating_sub(VALUE));
            let (shown, at) = match focused {
                true => super::super::composehead::window(&f.text, f.at, room),
                false => (f.text.clone(), 0),
            };
            let text = match (f.text.is_empty(), mark) {
                (true, _) => {
                    let at = VALUE + u16::from(focused);
                    cv.text(row, at, end, empty, t.text_muted, false);
                    String::new()
                }
                (false, "") => shown,
                (false, m) => format!("{m}{shown}"),
            };
            cv.text(row, VALUE, end, &text, t.ink, false);
            if focused {
                let col = VALUE + (at + mark.len() * usize::from(!f.text.is_empty())) as u16;
                cv.text(
                    row,
                    col,
                    end + 1,
                    "\u{258f}",
                    crate::palette::accent(),
                    false,
                );
            }
            // Beside the words: why crew wrote them, or what a typed due reads as.
            let said = match (c.kept(line), line) {
                (true, _) => c.why[i].clone(),
                (false, Line::Due) if !f.text.trim().is_empty() => match due_of(&f.text) {
                    Some(d) => format!("\u{2192} {}", d.due.format("%a %b %-d")),
                    None => "not a date /todo reads".into(),
                },
                _ => String::new(),
            };
            let after = VALUE + crate::chatwidth::str_w(&text) as u16 + 2;
            if !said.is_empty() && !f.text.is_empty() && after + 4 < end {
                cv.text(
                    row,
                    after,
                    end,
                    &format!("\u{b7} {said}"),
                    t.text_muted,
                    false,
                );
            }
        }
    }
}

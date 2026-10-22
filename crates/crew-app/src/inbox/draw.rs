//! The inbox drawn: the thread list, or the reader, or the setup page, over
//! a foot that says what every key does, and what just happened.
use crew_render::CellView;
use unicode_width::UnicodeWidthChar;

use super::cards::Rect;
use super::{InboxPane, Stage};
use crate::chatwidth::str_w;

/// Cells written one run at a time, clipped to the pane, never past `cols-1`.
pub(crate) struct Canvas {
    pub out: Vec<CellView>,
    pub cols: u16,
    pub rows: u16,
}

impl Canvas {
    pub(crate) fn new(cols: u16, rows: u16) -> Self {
        Self {
            out: Vec::new(),
            cols,
            rows,
        }
    }

    /// `s` from `col` on `row`, ellipsized at `end` (exclusive).
    pub(crate) fn text(
        &mut self,
        row: u16,
        col: u16,
        end: u16,
        s: &str,
        fg: (u8, u8, u8),
        bold: bool,
    ) {
        if row >= self.rows || col >= end {
            return;
        }
        let from = self.out.len();
        crate::navtext::put_at(
            &mut self.out,
            s,
            col,
            row,
            end.min(self.cols.saturating_sub(1)),
            fg,
        );
        for c in &mut self.out[from..] {
            c.bold = bold;
        }
    }
}

/// Greedy word wrap to `width` display columns; a word longer than a line is
/// cut, and blank lines stay.
pub(crate) fn wrap(text: &str, width: usize) -> Vec<String> {
    let width = width.max(4);
    let mut out = Vec::new();
    for para in text.split('\n') {
        let mut line = String::new();
        let mut w = 0;
        for word in para.split(' ') {
            let ww: usize = word.chars().map(|c| c.width().unwrap_or(0)).sum();
            if w > 0 && w + 1 + ww > width {
                out.push(std::mem::take(&mut line));
                w = 0;
            }
            if w > 0 {
                line.push(' ');
                w += 1;
            }
            for c in word.chars() {
                let cw = c.width().unwrap_or(0);
                if w + cw > width {
                    out.push(std::mem::take(&mut line));
                    w = 0;
                }
                line.push(c);
                w += cw;
            }
        }
        out.push(line);
    }
    out
}

impl InboxPane {
    pub fn cells(&self, cols: u16, rows: u16) -> Vec<CellView> {
        // Everything but the foot draws in the rows above it.
        let mut cv = Canvas::new(cols, self.body_rows(cols, rows));
        match (&self.stage, &self.reading) {
            (Stage::Ready(_), reading) => {
                let k = self.cards(cols, rows);
                if let Some(b) = k.list {
                    super::cards::frame(&mut cv, b, "", "", reading.is_none());
                    let inner = b.inner();
                    let mut list = Canvas::new(inner.w, inner.h);
                    self.list(&mut list);
                    put(&mut cv, list, inner);
                }
                if let (Some(b), Some(r)) = (k.mail, reading) {
                    r.seen.set((cols, rows));
                    let counter = self.counter().unwrap_or_default();
                    let room = b.w.saturating_sub(8 + str_w(&counter) as u16);
                    let subject = super::reader::subject(r);
                    let keys = !self.carded();
                    super::cards::frame(&mut cv, b, &clip(subject, room), &counter, keys);
                    let inner = b.inner();
                    let mut read = Canvas::new(inner.w, inner.h);
                    super::reader::draw(&mut read, r, &self.pics);
                    put(&mut cv, read, inner);
                }
                if let Some(b) = k.reply {
                    let mut reply = Canvas::new(b.w + 1, b.h);
                    self.draw_compose(&mut reply, b.w);
                    self.draw_todo(&mut reply, b.w);
                    put(&mut cv, reply, b);
                }
            }
            // Before the list: a card of its own, as every section is.
            (Stage::Setup { found }, _) => {
                let kept = self.kept();
                page(&mut cv, self.cards(cols, rows).page, "set up Gmail", |c| {
                    super::setup::draw(c, found.as_ref(), kept.as_deref())
                })
            }
            (Stage::SigningIn(_), _) => {
                let b = self.cards(cols, rows).page;
                page(&mut cv, b, "signing in", super::setup::signing_in)
            }
        }
        cv.rows = rows;
        self.foot(&mut cv);
        cv.out
    }
}

/// The page's card (`cards`) named `name`, `fill` drawing inside it.
fn page(cv: &mut Canvas, b: Option<Rect>, name: &str, fill: impl FnOnce(&mut Canvas)) {
    let Some(b) = b else { return };
    super::cards::frame(cv, b, name, "", true);
    let inner = b.inner();
    let mut c = Canvas::new(inner.w, inner.h);
    fill(&mut c);
    put(cv, c, inner);
}

/// `inner`'s cells onto `cv`, where its rect is.
fn put(cv: &mut Canvas, inner: Canvas, at: Rect) {
    cv.out.extend(inner.out.into_iter().map(|c| CellView {
        col: c.col + at.col,
        row: c.row + at.row,
        ..c
    }));
}

/// `s` cut to `room` columns, an ellipsis where it was cut.
fn clip(s: &str, room: u16) -> String {
    let room = usize::from(room);
    if str_w(s) <= room {
        return s.to_string();
    }
    let mut out: String = String::new();
    for ch in s.chars() {
        if str_w(&out) + 2 > room {
            break;
        }
        out.push(ch);
    }
    format!("{out}\u{2026}")
}

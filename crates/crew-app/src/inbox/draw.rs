//! The inbox drawn: the thread list, or the reader, or the setup page, with
//! one line at the foot that says what the keys do — or what just happened.
use crew_render::CellView;
use unicode_width::UnicodeWidthChar;

use super::{InboxPane, Stage};

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
        let mut cv = Canvas::new(cols, rows);
        match (&self.stage, &self.reading) {
            (Stage::Ready(_), Some(r)) => super::reader::draw(&mut cv, r, &self.pics),
            (Stage::Ready(_), None) => self.list(&mut cv),
            (Stage::Setup { found }, _) => super::setup::draw(&mut cv, found.as_ref()),
            (Stage::SigningIn(_), _) => {
                let t = crew_theme::theme();
                cv.text(
                    1,
                    2,
                    cols,
                    "finish signing in in your browser\u{2026}",
                    t.ink,
                    false,
                );
                cv.text(
                    2,
                    2,
                    cols,
                    "(crew is listening for Google's answer)",
                    t.text_muted,
                    false,
                );
            }
        }
        self.foot(&mut cv);
        cv.out
    }
}

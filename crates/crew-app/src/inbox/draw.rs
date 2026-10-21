//! The inbox drawn: the thread list, or the reader, or the setup page, with
//! one line at the foot that says what the keys do — or what just happened.
use crew_render::CellView;
use unicode_width::UnicodeWidthChar;

use super::wire::{sender_name, Thread};
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

    fn list(&self, cv: &mut Canvas) {
        let t = crew_theme::theme();
        let room = cv.rows.saturating_sub(1) as usize;
        if self.rows.is_empty() {
            let said = match (self.busy, self.query.trim()) {
                (true, _) => "loading\u{2026}".to_string(),
                (false, "") => "nothing in the inbox".to_string(),
                (false, q) => format!("no mail matches \u{201c}{q}\u{201d}"),
            };
            cv.text(1, 2, cv.cols, &said, t.text_muted, false);
            return;
        }
        let top = self.sel.saturating_sub(room.saturating_sub(1));
        let now = chrono::Utc::now().timestamp_millis();
        for (i, th) in self.rows.iter().enumerate().skip(top).take(room) {
            row(cv, (i - top) as u16, th, i == self.sel, now);
        }
    }

    fn foot(&self, cv: &mut Canvas) {
        let t = crew_theme::theme();
        let last = cv.rows.saturating_sub(1);
        let (text, fg) = match (&self.typing, self.note.is_empty()) {
            (Some(q), _) => (format!("search: {q}\u{258f}"), crate::palette::accent()),
            (None, false) => (self.note.clone(), t.bell),
            (None, true) => (super::keys::hint(self), t.text_muted),
        };
        cv.text(last, 1, cv.cols, &text, fg, false);
    }
}

/// One thread: `›` when selected, `●` when unread, sender, subject and
/// snippet, and the age at the right edge.
fn row(cv: &mut Canvas, r: u16, th: &Thread, selected: bool, now: i64) {
    let t = crew_theme::theme();
    let accent = crate::palette::accent();
    let end = cv.cols.saturating_sub(1);
    if selected {
        cv.text(r, 0, end, "\u{203a}", accent, true);
    }
    if th.unread {
        cv.text(r, 1, end, "\u{25cf}", accent, false);
    }
    if th.starred {
        cv.text(r, 2, end, "\u{2605}", accent, false);
    }
    let age = super::age::said(th.at_ms, now);
    let age_at = end.saturating_sub(age.chars().count() as u16);
    cv.text(r, age_at, end, &age, t.text_muted, false);
    let sw = (cv.cols / 4).clamp(8, 22);
    let mut who = sender_name(&th.from);
    if th.count > 1 {
        who = format!("{who} ({})", th.count);
    }
    cv.text(r, 4, 4 + sw, &who, t.ink, th.unread || selected);
    let at = 4 + sw + 1;
    let subject = match th.subject.trim() {
        "" => "(no subject)",
        s => s,
    };
    let lead = format!("{subject} \u{2014} ");
    cv.text(r, at, age_at.saturating_sub(1), &lead, t.ink, th.unread);
    let snip_at = at + lead.chars().count() as u16;
    cv.text(
        r,
        snip_at,
        age_at.saturating_sub(1),
        &th.snippet,
        t.text_muted,
        false,
    );
}

//! The reply box drawn: a card under the open mail, the width of its text,
//! `reply to Ann Lee` (or `reply all`, `forward`) on its top edge as every
//! crew card wears its name. Inside, the `to` and `cc` lines, the words with
//! a caret where they go, and for a forward the mail it carries; the mail
//! above is given the rows it leaves. It grows with the words, up to half
//! the pane.
use super::compose::{Kind, On};
use super::draw::{wrap, Canvas};
use super::InboxPane;

/// Text rows the box shows at least.
const MIN_ROWS: u16 = 3;
/// The `to` and `cc` lines, and a row of air under them.
const HEAD: u16 = 3;

impl InboxPane {
    /// The rows the reply box takes in a pane `cols` by `rows`: none when
    /// there is no reply being written.
    pub(crate) fn compose_h(&self, cols: u16, rows: u16) -> u16 {
        let Some(c) = &self.compose else {
            return 0;
        };
        let (_, width) = self.reader_box(cols);
        let lines = wrap(&c.body.text, inner(width)).len() as u16;
        let carried = u16::from(c.forward.is_some());
        let least = MIN_ROWS + HEAD + carried + 2;
        let most = (self.body_rows(cols, rows) / 2).max(least);
        (lines.max(MIN_ROWS) + HEAD + carried + 2).min(most)
    }

    /// The rows the open mail draws in: above the foot and the reply box.
    pub(crate) fn reader_rows(&self, cols: u16, rows: u16) -> u16 {
        self.body_rows(cols, rows)
            .saturating_sub(self.compose_h(cols, rows))
    }

    /// The box, on its rows of `cv` (the reader's own canvas, `width` wide).
    pub(crate) fn draw_compose(&self, cv: &mut Canvas, top: u16, h: u16) {
        let Some(c) = &self.compose else {
            return;
        };
        let t = crew_theme::theme();
        let w = inner(cv.cols) as u16;
        let (left, right) = (1, 2 + w + 1);
        let legend = match (c.sending, c.kind) {
            (true, _) => format!(" sending to {}\u{2026} ", super::addr::names(&c.to.text)),
            (false, Kind::Reply) => format!(" reply to {} ", super::addr::names(&c.answer.to)),
            (false, Kind::All) => " reply all ".to_string(),
            (false, Kind::Forward) => " forward ".to_string(),
        };
        let rule = |l: char, r: char| {
            let mut s = String::from(l);
            s.extend(std::iter::repeat_n(
                '\u{2500}',
                usize::from(right - left - 1),
            ));
            s.push(r);
            s
        };
        let edge = t.border_normal;
        cv.text(
            top,
            left,
            cv.cols,
            &rule('\u{256d}', '\u{256e}'),
            edge,
            false,
        );
        cv.text(top, left + 2, right - 1, &legend, t.ink, true);
        let bottom = top + h - 1;
        cv.text(
            bottom,
            left,
            cv.cols,
            &rule('\u{2570}', '\u{256f}'),
            edge,
            false,
        );
        for r in top + 1..bottom {
            cv.text(r, left, left + 1, "\u{2502}", edge, false);
            cv.text(r, right, right + 1, "\u{2502}", edge, false);
        }
        super::composehead::draw(cv, c, top, right);
        let first_row = top + 1 + HEAD;
        let carried = u16::from(c.forward.is_some());
        let room = usize::from(bottom.saturating_sub(first_row + carried));
        let lines = wrap(&c.body.text, usize::from(w));
        let (row, col) = place(&c.body.text, c.body.at, usize::from(w));
        // The last rows of long words, with the caret's row in them.
        let first = row.saturating_sub(room.saturating_sub(1));
        if c.body.text.is_empty() && c.on != On::Body {
            cv.text(
                first_row,
                3,
                right - 1,
                super::composehead::hint(c, On::Body),
                t.text_muted,
                false,
            );
        }
        for (i, line) in lines.iter().enumerate().skip(first).take(room) {
            cv.text(
                first_row + (i - first) as u16,
                3,
                right - 1,
                line,
                t.ink,
                false,
            );
        }
        if c.on == On::Body {
            if c.body.text.is_empty() {
                cv.text(
                    first_row,
                    4,
                    right - 1,
                    super::composehead::hint(c, On::Body),
                    t.text_muted,
                    false,
                );
            }
            super::composehead::caret(
                cv,
                c,
                (first_row + (row - first) as u16, 3 + col as u16),
                right,
            );
        }
        if let Some(f) = &c.forward {
            cv.text(bottom - 1, 3, right - 1, &carrying(f), t.text_muted, false);
        }
    }
}

/// `↪ Ann Lee · Lunch · report.pdf, photo.jpg`: the mail a forward carries.
fn carrying(f: &super::forward::Forwarded) -> String {
    let mut s = format!(
        "\u{21aa} {} \u{b7} {}",
        super::wire::sender_name(&f.from),
        f.subject.trim()
    );
    if !f.files.is_empty() {
        let names: Vec<&str> = f.files.iter().map(|p| p.name.as_str()).collect();
        s.push_str(&format!(" \u{b7} {}", names.join(", ")));
    }
    s
}

/// The text's width inside a box in a reader `cols` wide.
fn inner(cols: u16) -> usize {
    super::reader::text_width(cols).saturating_sub(3)
}

/// The caret's wrapped row and column: where the text up to it ends.
fn place(text: &str, at: usize, width: usize) -> (usize, usize) {
    let before: String = text.chars().take(at).collect();
    let lines = wrap(&before, width);
    let last = lines.last().map_or(0, |l| crate::chatwidth::str_w(l));
    (lines.len().saturating_sub(1), last)
}

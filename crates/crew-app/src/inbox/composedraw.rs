//! The reply box drawn: a card under the open mail, the width of its text,
//! `reply to Ann Lee` on its top edge as every crew card wears its name, the
//! words inside with a caret where they go, and the mail above given the
//! rows it leaves. It grows with the reply, up to half the pane.
use super::draw::{wrap, Canvas};
use super::InboxPane;

/// Text rows the box shows at least.
const MIN_ROWS: u16 = 3;

impl InboxPane {
    /// The rows the reply box takes in a pane `cols` by `rows`: none when
    /// there is no reply being written.
    pub(crate) fn compose_h(&self, cols: u16, rows: u16) -> u16 {
        let Some(c) = &self.compose else {
            return 0;
        };
        let (_, width) = self.reader_box(cols);
        let lines = wrap(&c.text, inner(width)).len() as u16;
        let most = (self.body_rows(cols, rows) / 2).max(MIN_ROWS + 2);
        (lines.max(MIN_ROWS) + 2).min(most)
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
        let who = super::wire::sender_name(&c.answer.to);
        let legend = match c.sending {
            true => format!(" sending to {who}\u{2026} "),
            false => format!(" reply to {who} "),
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
        let lines = wrap(&c.text, usize::from(w));
        let (row, col) = caret(&c.text, c.at, usize::from(w));
        // The last rows of a long reply, with the caret's row in them.
        let room = usize::from(h.saturating_sub(2));
        let first = row.saturating_sub(room.saturating_sub(1));
        for r in top + 1..bottom {
            cv.text(r, left, left + 1, "\u{2502}", edge, false);
            cv.text(r, right, right + 1, "\u{2502}", edge, false);
        }
        if c.text.is_empty() {
            let hint = "write your reply \u{b7} tab: agent smith drafts it";
            cv.text(top + 1, 3, right - 1, hint, t.text_muted, false);
        }
        for (i, line) in lines.iter().enumerate().skip(first).take(room) {
            cv.text(
                top + 1 + (i - first) as u16,
                3,
                right - 1,
                line,
                t.ink,
                false,
            );
        }
        if !c.sending {
            let at = (top + 1 + (row - first) as u16, 3 + col as u16);
            cv.text(
                at.0,
                at.1,
                right,
                "\u{258f}",
                crate::palette::accent(),
                false,
            );
        }
    }
}

/// The text's width inside a box in a reader `cols` wide.
fn inner(cols: u16) -> usize {
    super::reader::text_width(cols).saturating_sub(3)
}

/// The caret's wrapped row and column: where the text up to it ends.
fn caret(text: &str, at: usize, width: usize) -> (usize, usize) {
    let before: String = text.chars().take(at).collect();
    let lines = wrap(&before, width);
    let last = lines.last().map_or(0, |l| crate::chatwidth::str_w(l));
    (lines.len().saturating_sub(1), last)
}

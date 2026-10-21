//! The reply box drawn: a card under the open mail, the width of its text,
//! `reply to Ann Lee` (or `reply all`, `forward`) on its top edge as every
//! crew card wears its name. Inside, the `to`, `cc` and `bcc` lines
//! (`composehead`), the words with a caret where they go, the row of files
//! (`composefiles`), and for a forward the mail it carries; the mail above
//! is given the rows it leaves. It grows with the words, up to half the
//! pane.
use super::compose::{Compose, Kind, On};
use super::draw::{wrap, Canvas};
use super::InboxPane;

/// Text rows the box shows at least.
const MIN_ROWS: u16 = 3;
/// The `to`, `cc` and `bcc` lines, and a row of air under them.
const HEAD: u16 = 4;

/// Where the box's parts sit, from its top edge at `top`, `h` rows tall.
pub(crate) struct Geo {
    /// The words' first row, and how many they have.
    pub words: u16,
    pub room: u16,
    /// The row of files' first row.
    pub files: u16,
}

pub(crate) fn geo(c: &Compose, top: u16, h: u16, w: u16) -> Geo {
    let bottom = top + h - 1;
    let carried = u16::from(c.forward.is_some());
    let files = bottom.saturating_sub(carried + super::composefiles::rows(c, w));
    let words = top + 1 + HEAD;
    Geo {
        words,
        room: files.saturating_sub(words),
        files,
    }
}

impl InboxPane {
    /// The rows the reply box takes in a pane `cols` by `rows`: none when
    /// there is no reply being written.
    pub(crate) fn compose_h(&self, cols: u16, rows: u16) -> u16 {
        let Some(c) = &self.compose else {
            return 0;
        };
        let w = inner(self.reader_box(cols).1);
        let lines = wrap(&c.body.text, w).len() as u16;
        let rest =
            HEAD + super::composefiles::rows(c, w as u16) + u16::from(c.forward.is_some()) + 2;
        let most = (self.body_rows(cols, rows) / 2).max(MIN_ROWS + rest);
        (lines.max(MIN_ROWS) + rest).min(most)
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
        let right = 3 + w;
        frame(cv, c, top, h, right);
        super::composehead::draw(cv, c, top, right);
        let g = geo(c, top, h, w);
        let room = usize::from(g.room);
        let lines = wrap(&c.body.text, usize::from(w));
        let (row, col) = place(&c.body.text, c.body.at, usize::from(w));
        // The last rows of long words, with the caret's row in them.
        let first = row.saturating_sub(room.saturating_sub(1));
        for (i, line) in lines.iter().enumerate().skip(first).take(room) {
            let r = g.words + (i - first) as u16;
            cv.text(r, 3, right - 1, line, t.ink, false);
        }
        if c.body.text.is_empty() {
            let at = 3 + u16::from(c.on == On::Body);
            let hint = super::composehead::hint(c, On::Body);
            cv.text(g.words, at, right - 1, hint, t.text_muted, false);
        }
        if c.on == On::Body {
            let at = (g.words + (row - first) as u16, 3 + col as u16);
            super::composehead::caret(cv, c, at, right);
        }
        super::composefiles::draw(cv, c, g.files, w);
        if let Some(f) = &c.forward {
            cv.text(top + h - 2, 3, right - 1, &carrying(f), t.text_muted, false);
        }
    }
}

/// The card's edges, and its name on the top one.
fn frame(cv: &mut Canvas, c: &Compose, top: u16, h: u16, right: u16) {
    let t = crew_theme::theme();
    let left = 1;
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
    let (edge, bottom) = (t.border_normal, top + h - 1);
    cv.text(
        top,
        left,
        cv.cols,
        &rule('\u{256d}', '\u{256e}'),
        edge,
        false,
    );
    cv.text(top, left + 2, right - 1, &legend, t.ink, true);
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
pub(crate) fn inner(cols: u16) -> usize {
    super::reader::text_width(cols).saturating_sub(3)
}

/// The caret's wrapped row and column: where the text up to it ends.
fn place(text: &str, at: usize, width: usize) -> (usize, usize) {
    let before: String = text.chars().take(at).collect();
    let lines = wrap(&before, width);
    let last = lines.last().map_or(0, |l| crate::chatwidth::str_w(l));
    (lines.len().saturating_sub(1), last)
}

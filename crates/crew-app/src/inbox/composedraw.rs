//! The reply box drawn: a card under the open mail's, as wide, `reply to Ann
//! Lee` (or `forward`) on its top edge as every crew card wears its name.
//! Inside, the `to` and `cc` lines (`composehead`), the words with a caret
//! where they go, the row of files (`composefiles`), and for a forward the
//! mail it carries. It grows with the words to a third of the pane, never
//! as tall as the mail's card (the user: "mail panel should have tallest
//! height").
use super::compose::{Kind, On};
use super::composegeo::geo;
use super::draw::{wrap, Canvas};
use super::InboxPane;

/// Text rows the box shows at least.
const MIN_ROWS: u16 = 3;
/// The `to` and `cc` lines, and a row of air under them.
const HEAD: u16 = 3;
/// Where the box's words start: its edge and two columns of air, under its
/// name and in line with the mail's words above.
pub(crate) const X: u16 = 3;

impl InboxPane {
    /// The rows the reply box takes in a pane `cols` by `rows`, its edges
    /// included: none when there is no reply being written. It grows with
    /// the words to a third of the pane, and never to the mail's height.
    pub(crate) fn compose_h(&self, cols: u16, rows: u16) -> u16 {
        let Some(c) = &self.compose else {
            return 0;
        };
        let w = text_w(self.mail_cols(cols).1);
        let lines = wrap(&c.body.text, w).len() as u16;
        let carried = u16::from(c.forward.is_some());
        let rest = HEAD + super::composefiles::rows(c, w as u16) + carried + 2;
        let body = self.body_rows(cols, rows);
        let most = (body / 3)
            .max(MIN_ROWS + rest)
            .min(body.saturating_sub(1) / 2);
        (lines.max(MIN_ROWS) + rest).min(most)
    }

    /// The rows the open mail's words draw in: inside its card, above the
    /// reply box and the foot.
    pub(crate) fn reader_rows(&self, cols: u16, rows: u16) -> u16 {
        let card = self.body_rows(cols, rows) - self.compose_h(cols, rows);
        card.saturating_sub(2)
    }

    /// The box on `cv`, a canvas of its own a column wider than the card
    /// (`w` wide), so the card's right edge is not the canvas's last column.
    pub(crate) fn draw_compose(&self, cv: &mut Canvas, w: u16) {
        let Some(c) = &self.compose else {
            return;
        };
        let t = crew_theme::theme();
        let h = cv.rows;
        // Named for whom the `to` line says now, as it is edited.
        let who = super::addr::names(&c.to.text);
        let legend = match (c.sending, c.kind, who.is_empty()) {
            (true, ..) => format!("sending to {who}\u{2026}"),
            (false, Kind::Reply, true) => "reply".to_string(),
            (false, Kind::Reply, false) => format!("reply to {who}"),
            (false, Kind::Forward, true) => "forward".to_string(),
            (false, Kind::Forward, false) => format!("forward to {who}"),
        };
        let b = super::cards::Rect {
            col: 0,
            row: 0,
            w,
            h,
        };
        super::cards::frame(cv, b, &legend, "", true);
        let tw = text_w(w) as u16;
        let end = X + tw;
        let g = geo(c, 0, h, tw);
        super::composehead::draw(cv, c, &g, end);
        let room = usize::from(g.room);
        let lines = wrap(&c.body.text, usize::from(tw));
        let (row, col) = place(&c.body.text, c.body.at, usize::from(tw));
        // The last rows of long words, with the caret's row in them.
        let first = row.saturating_sub(room.saturating_sub(1));
        for (i, line) in lines.iter().enumerate().skip(first).take(room) {
            cv.text(g.words + (i - first) as u16, X, end, line, t.ink, false);
        }
        if c.body.text.is_empty() && g.room > 0 {
            let at = X + u16::from(c.on == On::Body);
            let hint = super::composehead::hint(c, On::Body);
            cv.text(g.words, at, end, hint, t.text_muted, false);
        }
        if c.on == On::Body && g.room > 0 {
            let at = (g.words + (row - first) as u16, X + col as u16);
            super::composehead::caret(cv, c, at, end);
        }
        if let Some(top) = g.files {
            super::composefiles::draw(cv, c, top, tw);
        }
        if let Some(f) = &c.forward {
            cv.text(h - 2, X, end, &carrying(f), t.text_muted, false);
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

/// The words' width inside a card `w` wide: its edges and a column of air
/// each side, up to the reading measure.
pub(crate) fn text_w(w: u16) -> usize {
    usize::from(w.saturating_sub(2 * X)).min(super::reader::MEASURE)
}

/// The caret's wrapped row and column: where the text up to it ends.
pub(crate) fn place(text: &str, at: usize, width: usize) -> (usize, usize) {
    let before: String = text.chars().take(at).collect();
    let lines = wrap(&before, width);
    let last = lines.last().map_or(0, |l| crate::chatwidth::str_w(l));
    (lines.len().saturating_sub(1), last)
}

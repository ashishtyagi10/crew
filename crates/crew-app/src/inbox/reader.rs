//! An opened thread, read: each message's sender (and address, and time at
//! the right) over its words, oldest first, with air between messages; the
//! subject is on the card's edge (`cards`). The words are markdown drawn the way chat
//! draws it (`chatmd`), so a mail's headings, emphasis, lists, quotes,
//! tables and links survive (goal 2026-10-20-mail-reader, phase 1); quoted
//! replies fold to one row until `q`, and `v` reads the other version.
use std::sync::Arc;

use super::draw::Canvas;
use super::pics::{Pic, Pics};
use super::readmail::{Lay, Placed};
use super::Reading;
use crate::chatbody::{plain, CardLine, Color};

/// The thread laid out: card lines, one column of indent included, where
/// its pictures are, and which rows are folded mails (`readfold`).
pub(crate) struct Laid {
    pub lines: Vec<CardLine>,
    pub pics: Vec<Placed>,
    pub folded: Vec<(usize, usize)>,
}

/// Everything a layout depends on besides the thread itself.
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct Key {
    pub(crate) width: usize,
    plain: bool,
    quotes: bool,
    web: bool,
    loaded: bool,
    /// Pictures landed and sized (`readfit`).
    sized: usize,
    /// Mails folded or opened (`readfold`).
    folds: u32,
    pub(crate) ink: Color,
    pub(crate) muted: Color,
    /// The invite card's bar (`ics::card`).
    pub(crate) accent: Color,
}

impl Reading {
    /// The layout at `width` text columns, built once per change.
    pub(crate) fn laid(&self, width: usize) -> Arc<Laid> {
        let t = crew_theme::theme();
        let key = Key {
            width: width.max(8),
            plain: self.plain,
            quotes: self.quotes,
            web: self.web,
            loaded: self.mails.is_some() || self.failed.is_some(),
            sized: self.sizes.len(),
            folds: self.folds,
            ink: t.ink,
            muted: t.text_muted,
            accent: crate::palette::accent(),
        };
        let mut cache = self.laid.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((_, laid)) = cache.as_ref().filter(|(k, _)| *k == key) {
            return Arc::clone(laid);
        }
        let laid = Arc::new(layout(self, key));
        *cache = Some((key, Arc::clone(&laid)));
        laid
    }
}

/// How many rows the open thread takes in a pane `cols` wide.
pub(crate) fn height(r: &Reading, cols: u16) -> usize {
    r.laid(text_width(cols)).lines.len()
}

/// The text's width in a reader `cols` wide: all of it but the margins, up
/// to a reading measure. On a wide pane a line ran the width of the screen;
/// a mail is set for about 600 px, some 75 columns.
pub(crate) fn text_width(cols: u16) -> usize {
    (cols.saturating_sub(4) as usize).min(MEASURE)
}

/// The longest line the reader sets.
pub(crate) const MEASURE: usize = 88;

/// The thread's subject, as its card's edge names it.
pub(crate) fn subject(r: &Reading) -> &str {
    match r.thread.subject.trim() {
        "" => "(no subject)",
        s => s,
    }
}

fn layout(r: &Reading, k: Key) -> Laid {
    let mut lay = Lay {
        sizes: r.sizes.clone(),
        ..Lay::default()
    };
    match &r.mails {
        Some(mails) => {
            let w = super::readfold::names_w(mails);
            for (i, m) in mails.iter().enumerate() {
                match r.folded(i, mails.len()) {
                    // The folded lines run together under a row of air.
                    true if i == 0 => {
                        lay.lines.push(Vec::new());
                        super::readfold::line(&mut lay, m, &k, i, w);
                    }
                    true => super::readfold::line(&mut lay, m, &k, i, w),
                    false => super::readmail::message(&mut lay, m, r, &k, i == 0),
                }
            }
        }
        None => {
            lay.lines.push(Vec::new());
            match &r.failed {
                // Esc and Enter ask for it again (`open_at`).
                Some(e) => {
                    let said = format!("could not open this thread: {e}");
                    for line in super::draw::wrap(&said, k.width.max(1)) {
                        lay.lines.push(row(&line, k.ink, false));
                    }
                    lay.lines
                        .push(row("esc, then enter tries again", k.muted, false));
                }
                None => lay.lines.push(row("loading\u{2026}", k.muted, false)),
            }
        }
    }
    Laid {
        lines: lay.lines,
        pics: lay.pics,
        folded: lay.folded,
    }
}

/// One line of the reader's own words, indented like the markdown's.
pub(crate) fn row(text: &str, fg: Color, bold: bool) -> CardLine {
    std::iter::once(' ')
        .chain(text.chars())
        .map(|c| plain(c, fg, bold))
        .collect()
}

pub(crate) fn draw(cv: &mut Canvas, r: &Reading, pics: &Pics) {
    let t = crew_theme::theme();
    let room = usize::from(cv.rows);
    let laid = r.laid(text_width(cv.cols));
    for (i, line) in laid.lines.iter().skip(r.scroll).take(room).enumerate() {
        let mut cells =
            crate::chatplace::line_cells(i as u16, line, cv.cols.saturating_sub(2), t.page_bg);
        for c in &mut cells {
            c.col += 1;
        }
        cv.out.extend(cells);
    }
    // A picture not drawn yet says so in its box; a drawn one is paint.
    for p in &laid.pics {
        let say = match pics.get(&p.key) {
            Some(Pic::Ready(_)) => continue,
            Some(Pic::Failed) if p.alt.is_empty() => "\u{25a3} (a picture)".to_string(),
            Some(Pic::Failed) => format!("\u{25a3} {}", p.alt),
            _ => "loading picture\u{2026}".to_string(),
        };
        if let Some(y) = p.row.checked_sub(r.scroll).filter(|&y| y < room) {
            cv.text(y as u16, 2, cv.cols, &say, t.text_muted, false);
        }
    }
}

/// A `Date` header in local time, `Tue Oct 20, 09:14`; as written when it
/// does not parse. Mailers add a zone comment (`+0000 (UTC)`) or a weekday
/// that disagrees with the date, and RFC 2822 parsing refuses both, so each
/// is tried without them too.
pub(crate) fn when(date: &str) -> String {
    super::days::sent(date).map_or_else(
        || date.trim().to_string(),
        |t| t.format("%a %b %-d, %H:%M").to_string(),
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_date_header_reads_in_local_time_whatever_the_mailer_added() {
        for d in [
            "Tue, 20 Oct 2026 13:02:00 +0000",
            "Tue, 20 Oct 2026 13:02:00 +0000 (UTC)",
            "Mon, 20 Oct 2026 13:02:00 +0000",
        ] {
            let got = super::when(d);
            assert!(
                got.contains("Oct 20") || got.contains("Oct 21"),
                "{d} -> {got}"
            );
        }
        assert_eq!(super::when("  yesterday "), "yesterday");
    }
}

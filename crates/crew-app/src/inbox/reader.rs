//! An opened thread, read: the subject, then each message's sender and time
//! over its words, oldest first. The words are markdown drawn the way chat
//! draws it (`chatmd`), so a mail's headings, emphasis, lists, quotes,
//! tables and links survive (goal 2026-10-20-mail-reader, phase 1); quoted
//! history folds to one row until `z`, and `v` reads the other version.
use std::sync::Arc;

use super::draw::{wrap, Canvas};
use super::pics::{Pic, Pics};
use super::readmail::{Lay, Placed};
use super::Reading;
use crate::chatbody::{plain, CardLine, Color};

/// The thread laid out: card lines, one column of indent included, and
/// where its pictures are.
pub(crate) struct Laid {
    pub lines: Vec<CardLine>,
    pub pics: Vec<Placed>,
}

/// Everything a layout depends on besides the thread itself.
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct Key {
    pub(crate) width: usize,
    plain: bool,
    quotes: bool,
    web: bool,
    loaded: bool,
    pub(crate) ink: Color,
    pub(crate) muted: Color,
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
            loaded: self.mails.is_some(),
            ink: t.ink,
            muted: t.text_muted,
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

pub(crate) fn text_width(cols: u16) -> usize {
    cols.saturating_sub(4) as usize
}

fn layout(r: &Reading, k: Key) -> Laid {
    let subject = match r.thread.subject.trim() {
        "" => "(no subject)",
        s => s,
    };
    let mut lay = Lay::default();
    lay.lines = wrap(subject, k.width)
        .iter()
        .map(|l| row(l, k.ink, true))
        .collect();
    match &r.mails {
        Some(mails) => mails
            .iter()
            .for_each(|m| super::readmail::message(&mut lay, m, r, &k)),
        None => {
            lay.lines.push(Vec::new());
            lay.lines.push(row("loading\u{2026}", k.muted, false));
        }
    }
    Laid {
        lines: lay.lines,
        pics: lay.pics,
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
    let room = cv.rows.saturating_sub(1) as usize;
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
    let d = date.trim();
    let bare = d.split(" (").next().unwrap_or(d);
    let dayless = bare.split_once(", ").map_or(bare, |(_, rest)| rest);
    [d, bare, dayless]
        .iter()
        .find_map(|s| chrono::DateTime::parse_from_rfc2822(s).ok())
        .map(|t| {
            t.with_timezone(&chrono::Local)
                .format("%a %b %-d, %H:%M")
                .to_string()
        })
        .unwrap_or_else(|| d.to_string())
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

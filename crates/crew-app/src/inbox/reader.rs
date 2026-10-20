//! An opened thread, read: the subject, then each message's sender and time
//! over its words, oldest first. The words are markdown drawn the way chat
//! draws it (`chatmd`), so a mail's headings, emphasis, lists, quotes,
//! tables and links survive (goal 2026-10-20-mail-reader, phase 1); quoted
//! history folds to one row until `z`, and `v` reads the other version.
use std::sync::Arc;

use super::draw::{wrap, Canvas};
use super::rich::Rich;
use super::wire::sender_name;
use super::Reading;
use crate::chatbody::{plain, CardLine, Color};

/// The thread laid out: card lines, one column of indent included.
pub(crate) struct Laid {
    pub lines: Vec<CardLine>,
}

/// Everything a layout depends on besides the thread itself.
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct Key {
    width: usize,
    plain: bool,
    quotes: bool,
    loaded: bool,
    ink: Color,
    muted: Color,
}

impl Reading {
    /// The layout at `width` text columns, built once per change.
    pub(crate) fn laid(&self, width: usize) -> Arc<Laid> {
        let t = crew_theme::theme();
        let key = Key {
            width: width.max(8),
            plain: self.plain,
            quotes: self.quotes,
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
    let mut out: Vec<CardLine> = wrap(subject, k.width)
        .iter()
        .map(|l| row(l, k.ink, true))
        .collect();
    let Some(mails) = &r.mails else {
        out.push(Vec::new());
        out.push(row("loading\u{2026}", k.muted, false));
        return Laid { lines: out };
    };
    for m in mails {
        // A mail built from its text alone (an older grant's, a test's).
        let own =
            (m.rich.body.is_empty() && !m.text.is_empty()).then(|| Rich::of_plain(&m.text, None));
        let rich = own.as_ref().unwrap_or(&m.rich);
        let rich = match (r.plain, &rich.alt) {
            (true, Some(alt)) => alt.as_ref(),
            _ => rich,
        };
        out.push(Vec::new());
        let who = format!("{} \u{b7} {}", sender_name(&m.from), when(&m.date));
        out.push(row(&who, k.muted, true));
        out.extend(md(&rich.body, k.width, k.ink));
        if let Some(q) = &rich.quote {
            let lines = md(q, k.width, k.ink);
            match r.quotes {
                true => out.extend(lines),
                false => {
                    let n = lines.len();
                    let fold =
                        format!("\u{b7}\u{b7}\u{b7} quoted text ({n} lines) \u{b7} z shows it");
                    out.push(row(&fold, k.muted, false));
                }
            }
        }
        if let Some(sig) = &rich.sig {
            out.push(Vec::new());
            out.extend(md(sig, k.width, k.muted));
        }
        let files = (!m.files.is_empty()).then(|| format!("attached: {}", m.files.join(", ")));
        for quiet in [rich.note.clone(), files].into_iter().flatten() {
            out.extend(wrap(&quiet, k.width).iter().map(|l| row(l, k.muted, false)));
        }
    }
    Laid { lines: out }
}

/// Markdown as card lines at `width` columns.
fn md(text: &str, width: usize, fg: Color) -> Vec<CardLine> {
    let rendered = crate::md::render_chat(text, width);
    crate::chatmd::with_pictures(rendered, width, fg).0
}

/// One line of the reader's own words, indented like the markdown's.
fn row(text: &str, fg: Color, bold: bool) -> CardLine {
    std::iter::once(' ')
        .chain(text.chars())
        .map(|c| plain(c, fg, bold))
        .collect()
}

pub(crate) fn draw(cv: &mut Canvas, r: &Reading) {
    let page = crew_theme::theme().page_bg;
    let room = cv.rows.saturating_sub(1) as usize;
    let laid = r.laid(text_width(cv.cols));
    for (i, line) in laid.lines.iter().skip(r.scroll).take(room).enumerate() {
        let mut cells =
            crate::chatplace::line_cells(i as u16, line, cv.cols.saturating_sub(2), page);
        for c in &mut cells {
            c.col += 1;
        }
        cv.out.extend(cells);
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

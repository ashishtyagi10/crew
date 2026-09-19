//! The plain rung: prose or a listing, no gutter, wrapped on words. Split
//! from [`super::linepaint`], whose rungs are numbered and wrap wherever the
//! column runs out — right for code, wrong for a sentence.
use super::linepaint::{row, row_paint};
use crate::chatbody::{plain, CardLine};
use crate::viewpane::codepaint::{line_paint, CharPaint};

/// The mark a continued row wears, and the columns it costs.
///
/// The plain rung is the one rung with NO gutter, so a wrapped row had
/// nothing to its left saying it was one: in `/watching` at a tile width,
/// `w1  in 2h  daily  brief me on the` was followed by `calendar` in column
/// zero, which reads as the next standing intent rather than the end of this
/// one. The numbered rungs have answered this since they were written — a
/// `↪` where the line number would be — and so does a fenced block in a chat
/// card; with no gutter to put it in, it goes at the head of the row.
const MARK: char = '\u{21aa}';
const MARK_W: usize = 2;

/// Columns a continued row needs for its words before the mark is worth
/// paying for. Under this the mark starts hard-cutting words that used to fit
/// whole, and a wrap you can see is not worth a word you cannot read.
const MIN_TEXT: usize = 12;

/// Plain rows with no gutter at all, for text that is not source: a note, a
/// listing crew wrote itself. Wrapped at the full width and ON WORDS — a
/// sentence broken mid-word is how source is shown, because a line of code
/// has no words to break on; prose does — with a continuation keeping the
/// line's own indent and wearing a [`MARK`], so a detail line stays under its
/// row and a row that wrapped cannot be read as the next one.
pub(crate) fn unnumbered(
    text: &str,
    cols: usize,
    ink: (u8, u8, u8),
    muted: (u8, u8, u8),
    ws: &[Vec<bool>],
) -> Vec<CardLine> {
    let mut paints: Vec<Vec<CharPaint>> = text
        .split('\n')
        .map(|line| line_paint(line, "", ink))
        .collect();
    super::whitespace::dim(&mut paints, ws, muted);
    let cols = cols.max(1);
    let mut out = Vec::new();
    for (i, line) in text.split('\n').enumerate() {
        let chars: Vec<char> = line.chars().collect();
        let lead = chars
            .iter()
            .take_while(|c| **c == ' ')
            .count()
            .min(cols / 2);
        // The mark is paid for out of the row's words, so it is only worn
        // where there are still words enough to read (see [`MIN_TEXT`]).
        let marked = cols.saturating_sub(lead + MARK_W) >= MIN_TEXT;
        let cont = lead + if marked { MARK_W } else { 0 };
        let mut pieces = hanging(&chars, cols, cont);
        // A first piece that is only the line's indent is not a row: an
        // unbreakable word after four spaces wrapped as a blank row and then
        // the word, which reads as a gap in the listing.
        if pieces.len() > 1 && chars[pieces[0].0..pieces[0].1].iter().all(|c| *c == ' ') {
            pieces.remove(0);
        }
        let mut first = true;
        for (s, e) in pieces {
            let (s, e) = (s.min(chars.len()), e.min(chars.len()).max(s));
            let mut row = match first {
                true => row("", ink, false),
                false => {
                    let mut r = row(&" ".repeat(lead), ink, false);
                    if marked {
                        r.push(plain(MARK, muted, false));
                        r.push(plain(' ', muted, false));
                    }
                    r
                }
            };
            let body = &chars[s..e];
            match row_paint(&paints, i + 1, s, body.len()) {
                Some(paint) => row.extend(
                    body.iter()
                        .zip(paint)
                        .map(|(c, (fg, bold))| plain(*c, *fg, *bold)),
                ),
                None => row.extend(body.iter().map(|c| plain(*c, ink, false))),
            }
            debug_assert!(row.len() <= cols, "a hanging wrap fits by construction");
            out.push(row);
            first = false;
        }
    }
    out
}

/// Word-wrap `chars` with a hanging indent: the first piece gets the whole
/// width, every later piece `cols - lead`, so the indent a continuation is
/// drawn with never pushes its tail past the edge.
///
/// This used to wrap everything at `cols` and then TRUNCATE each indented
/// continuation back to `cols` — up to `lead` characters of every wrapped
/// detail line in `/tools`, `/log` and any indented prose in `/view` were
/// silently gone, with no mark, and the row looked complete.
pub(crate) fn hanging(chars: &[char], cols: usize, lead: usize) -> Vec<(usize, usize)> {
    let mut out = crate::chatlayout::wrap_indices(chars, cols);
    let Some(&(_, e)) = out.first() else {
        return out;
    };
    if e >= chars.len() || lead == 0 {
        return out;
    }
    // Past the first piece; a word break also consumed its space.
    let next = e + usize::from(chars.get(e) == Some(&' '));
    out.truncate(1);
    if next >= chars.len() {
        return out;
    }
    let rest = crate::chatlayout::wrap_indices(&chars[next..], cols.saturating_sub(lead).max(1));
    out.extend(rest.into_iter().map(|(s, e)| (s + next, e + next)));
    out
}

#[cfg(test)]
#[path = "plainrung_tests.rs"]
mod tests;

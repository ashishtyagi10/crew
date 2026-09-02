//! The plain rung: prose or a listing, no gutter, wrapped on words. Split
//! from [`super::linepaint`], whose rungs are numbered and wrap wherever the
//! column runs out — right for code, wrong for a sentence.
use super::linepaint::{row, row_paint};
use crate::chatbody::{plain, CardLine};
use crate::viewpane::codepaint::{line_paint, CharPaint};

/// Plain rows with no gutter at all, for text that is not source: a note, a
/// listing crew wrote itself. Wrapped at the full width and ON WORDS — a
/// sentence broken mid-word is how source is shown, because a line of code
/// has no words to break on; prose does — with a continuation keeping the
/// line's own indent, so a detail line stays under its row.
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
        let mut first = true;
        for (s, e) in crate::chatlayout::wrap_indices(&chars, cols) {
            let indent = if first { 0 } else { lead };
            let (s, e) = (s.min(chars.len()), e.min(chars.len()).max(s));
            let mut row = row(&" ".repeat(indent), ink, false);
            let body = &chars[s..e];
            match row_paint(&paints, i + 1, s, body.len()) {
                Some(paint) => row.extend(
                    body.iter()
                        .zip(paint)
                        .map(|(c, (fg, bold))| plain(*c, *fg, *bold)),
                ),
                None => row.extend(body.iter().map(|c| plain(*c, ink, false))),
            }
            // A continuation with its indent could overrun the width by the
            // indent; the cells past it are dropped rather than wrapped again.
            row.truncate(cols);
            out.push(row);
            first = false;
        }
    }
    out
}

#[cfg(test)]
#[path = "plainrung_tests.rs"]
mod tests;

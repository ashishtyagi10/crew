//! The two cell primitives `chatmd` lays a document out with: a styled
//! span's cells, and the chunking of a row to the card's display width.
//! Split from `chatmd` for the line cap.
use crate::chatbody::{plain, CardCell, CardLine, Color};
use crate::md::{LineKind, MdSpan};

/// Splits `cells` into rows of at most `width` DISPLAY columns (a wide glyph
/// counts two), each prefixed with a one-column indent cell.
pub(crate) fn push_chunked(
    out: &mut Vec<CardLine>,
    cells: &[CardCell],
    width: usize,
    line_fg: Color,
) {
    if cells.is_empty() {
        out.push(vec![plain(' ', line_fg, false)]);
        return;
    }
    let full: Vec<char> = cells.iter().map(|c| c.c).collect();
    let mut s = 0;
    loop {
        let e = crate::chatwidth::fit_end(&full, s, width);
        let mut row = vec![plain(' ', line_fg, false)];
        row.extend(cells[s..e].iter().cloned());
        out.push(row);
        s = e;
        if s >= full.len() {
            break;
        }
    }
}

/// Per-char cells for one styled span, given the line's kind (chrome/code
/// lines override span style entirely; body spans map `MdStyle`).
pub(crate) fn span_cells(span: &MdSpan, kind: LineKind, fg: Color, muted: Color) -> Vec<CardCell> {
    let ink = crate::chatspan::style(span, kind, fg, muted);
    // Each character carries the byte it came from: the span's offset plus
    // the bytes of the characters before it in the span.
    let mut at = span.src;
    span.text
        .chars()
        .map(|c| {
            let src = at;
            at = at.map(|n| n + c.len_utf8() as u32);
            CardCell {
                c,
                fg: ink.fg,
                bold: ink.bold,
                italic: ink.italic,
                strike: ink.strike,
                bg: ink.bg,
                link: ink.link.clone(),
                src,
                pic: None,
            }
        })
        .collect()
}

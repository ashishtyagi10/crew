//! Turns a fenced code block's mapped lines into a solid FIELD: one tinted
//! rectangle of uniform width, the language on its first row, a blank row to
//! close it.
//!
//! What it replaces drew as `╭─ rust`, then a background that stopped at the
//! end of each line, then a lone `╰─`. Two stub corners and a ragged right
//! edge are not a card — shot side by side with the prose around it, the
//! block read as an unfinished box rather than as a block, and on the darker
//! presets the tint was faint enough that the corners were the only thing
//! saying a fence was there at all. A rectangle needs no corners to be one.
use crate::chatbody::{CardCell, CardLine};

/// Columns of padding on each side of the code, inside the field. Code is
/// wrapped `2 * PAD` narrower than the card so the field always fits (see
/// `md::layout::code_block_lines`).
pub(crate) const PAD: usize = 1;

fn tinted(bg: (u8, u8, u8), fg: (u8, u8, u8)) -> CardCell {
    CardCell {
        c: ' ',
        fg,
        bold: false,
        italic: false,
        bg: Some(bg),
        link: None,
        src: None,
    }
}

/// Display width of `line` from column `from` on.
fn inner_w(line: &CardLine, from: usize) -> usize {
    line.iter()
        .skip(from)
        .map(|c| crate::chatwidth::char_w(c.c))
        .sum()
}

/// Where the field starts on this line: past the indent cell, and past a
/// blockquote's bar if the fence is inside one. The bar is prefixed to every
/// line of a quote — the code rows included — and a bar swallowed by the
/// field would make the quote stop reading as a quote at exactly the point
/// it got interesting. Read back off the ink it was given, the way
/// `diffrefine` reads its marks.
fn field_start(line: &CardLine) -> usize {
    let marker = crate::chatink::marker_fg();
    1 + line
        .iter()
        .skip(1)
        .take_while(|c| c.fg == marker && c.bg.is_none())
        .count()
}

/// Lay each `(start, end)` run of already-mapped code lines into a field
/// `width` columns wide at most. Every line in the run is padded to the same
/// width and every cell in it carries the code background, so the block is
/// one rectangle whatever its longest line is.
pub(crate) fn fill(lines: &mut [CardLine], runs: &[(usize, usize)], width: usize) {
    let bg = crate::chatink::code_bg();
    let fg = crate::chatink::code_fg();
    for &(start, end) in runs {
        // `width` is already the card's width past its one-column indent —
        // the indent cell keeps the page — so the whole of it is the field's.
        let lead = lines[start..end].iter().map(field_start).max().unwrap_or(1);
        let avail = width.saturating_sub(lead - 1).max(1);
        let content = lines[start..end]
            .iter()
            .map(|l| inner_w(l, lead))
            .max()
            .unwrap_or(0);
        let field = (content + PAD * 2).clamp(1, avail);
        for line in &mut lines[start..end] {
            for _ in 0..PAD {
                line.insert(lead.min(line.len()), tinted(bg, fg));
            }
            for cell in line.iter_mut().skip(lead) {
                cell.bg = Some(bg);
            }
            while inner_w(line, lead) < field {
                line.push(tinted(bg, fg));
            }
        }
    }
}

#[cfg(test)]
#[path = "chatfield_tests.rs"]
mod tests;

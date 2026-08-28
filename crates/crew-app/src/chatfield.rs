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
mod tests {
    use crate::chatbody::body_lines;

    fn text(line: &crate::chatbody::CardLine) -> String {
        line.iter().map(|c| c.c).collect()
    }

    /// The field is a rectangle: every row of the block the same width,
    /// whatever the longest line is. A background that stopped where each
    /// line did was the ragged edge this replaced.
    #[test]
    fn every_row_of_a_block_is_the_same_width() {
        let _g = crate::app::theme_test_guard();
        let lines = body_lines("```rs\nx\nlonger line here\ny\n```", 40, (9, 9, 9), false);
        let widths: Vec<usize> = lines.iter().map(|l| text(l).chars().count()).collect();
        assert!(
            widths.windows(2).all(|w| w[0] == w[1]),
            "{:?}",
            lines.iter().map(text).collect::<Vec<_>>()
        );
        // Widest line (16) + a pad each side + the card's indent column.
        assert_eq!(widths[0], 16 + super::PAD * 2 + 1);
    }

    /// A fence inside a blockquote keeps its bar on the page. The bar is
    /// prefixed to every line of the quote, code rows included, and a field
    /// that started at column 1 would eat it.
    #[test]
    fn a_quoted_fence_keeps_its_bar_off_the_field() {
        let _g = crate::app::theme_test_guard();
        let lines = body_lines("> ```\n> x = 1\n> ```", 40, (9, 9, 9), false);
        let bg = Some(crate::chatink::code_bg());
        let starts: Vec<usize> = lines
            .iter()
            .map(|l| l.iter().position(|c| c.bg == bg).expect("a field"))
            .collect();
        for (line, start) in lines.iter().zip(&starts) {
            assert!(*start >= 2, "the bar stays on the page: {:?}", text(line));
            assert!(line[..*start].iter().all(|c| c.bg.is_none()));
        }
        assert!(
            starts.windows(2).all(|w| w[0] == w[1]),
            "the field's left edge is one column on every row: {starts:?}"
        );
    }
}

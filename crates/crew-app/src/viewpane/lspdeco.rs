//! The curly underline under a diagnostic's range, laid onto the rendered
//! cells after the fact — the same post-pass shape as the selection wash in
//! `render`, but keyed on (line, column) rather than on source bytes, because
//! the code rungs carry no `src` on their cells (only markdown does).
//!
//! Columns need translating: a server counts UTF-16 units into the SOURCE
//! line, the viewer draws the line with its tabs expanded (see
//! [`super::whitespace`]), and a wide glyph takes two cells. So a span is
//! first put into rendered CHARS, then found on screen by walking each row's
//! cells and their widths.
use std::collections::HashMap;

use crew_render::CellView;
use crew_theme::deco::{Deco, DecoLine};

use crate::chatbody::{CardLine, Color};
use crate::viewpane::lines::GUTTER_W;
use crew_lsp::Diagnostic;

/// One line's worth of underline, in rendered-char offsets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Span {
    /// 0-based source line.
    pub line: usize,
    pub start: usize,
    pub end: usize,
    pub color: Color,
}

/// The rendered-char offset of UTF-16 column `col` in `line`: tabs count
/// for the spaces they became, a dropped `\r` for nothing.
pub(crate) fn rendered_offset(line: &str, col: u32, reveal: bool) -> usize {
    let (mut units, mut out, mut width) = (0u32, 0usize, 0usize);
    for c in line.chars() {
        if units >= col {
            break;
        }
        units += c.len_utf16() as u32;
        match c {
            '\t' => {
                let w = super::whitespace::TAB_STOP - (width % super::whitespace::TAB_STOP);
                out += w;
                width += w;
            }
            '\r' if !reveal => {}
            _ => {
                out += 1;
                width += crate::chatwidth::char_w(c);
            }
        }
    }
    out
}

/// Spans for every error and warning, one per line the range covers; an
/// empty range is widened to one character so it still shows.
pub(crate) fn spans(text: &str, diags: &[Diagnostic], reveal: bool) -> Vec<Span> {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut out = Vec::new();
    for d in diags {
        let Some((_, color)) = super::lspgutter::mark_for(d.severity) else {
            continue;
        };
        let (a, b) = (d.range.start, d.range.end);
        for n in (a.line as usize)..=(b.line as usize).min(lines.len().saturating_sub(1)) {
            let src = lines.get(n).copied().unwrap_or("");
            let start = if n == a.line as usize {
                rendered_offset(src, a.character, reveal)
            } else {
                0
            };
            let mut end = if n == b.line as usize {
                rendered_offset(src, b.character, reveal)
            } else {
                rendered_offset(src, u32::MAX, reveal)
            };
            if end <= start {
                end = start + 1;
            }
            out.push(Span {
                line: n,
                start,
                end,
                color,
            });
        }
    }
    out
}

/// What a rendered row is: the source line it starts, a wrap continuation
/// of the previous row's line, or something else (a banner).
enum Row {
    Starts(usize),
    Continues,
    Other,
}

fn kind(line: &CardLine, at: usize) -> Row {
    if let Some(n) = super::blamegutter::source_line_at(line, at) {
        return Row::Starts(n);
    }
    match line.get(at + GUTTER_W - 2).map(|c| c.c) {
        Some('\u{21aa}') => Row::Continues,
        _ => Row::Other,
    }
}

fn text_len(line: &CardLine, at: usize) -> usize {
    line.len().saturating_sub(at + GUTTER_W)
}

/// Underline `spans` on the cells of rows `top..top + rows`. `at` is the
/// width of the columns prepended before the number gutter.
pub(crate) fn apply(
    out: &mut [CellView],
    lines: &[CardLine],
    top: usize,
    rows: usize,
    at: usize,
    spans: &[Span],
) {
    if spans.is_empty() || top >= lines.len() {
        return;
    }
    // Where row `top` stands in its source line: a continuation's offset is
    // the text of the rows above it back to the numbered one.
    let (mut cur, mut pos) = match kind(&lines[top], at) {
        Row::Starts(n) => (Some(n), 0),
        Row::Continues => {
            let (mut n, mut pos, mut r) = (None, 0, top);
            while r > 0 {
                r -= 1;
                pos += text_len(&lines[r], at);
                match kind(&lines[r], at) {
                    Row::Starts(k) => {
                        n = Some(k);
                        break;
                    }
                    Row::Continues => {}
                    Row::Other => break,
                }
            }
            (n, pos)
        }
        Row::Other => (None, 0),
    };
    let index: HashMap<(u16, u16), usize> = out
        .iter()
        .enumerate()
        .map(|(i, c)| ((c.row, c.col), i))
        .collect();
    for (i, line) in lines.iter().skip(top).take(rows).enumerate() {
        if i > 0 {
            match kind(line, at) {
                Row::Starts(n) => (cur, pos) = (Some(n), 0),
                Row::Continues => {}
                Row::Other => cur = None,
            }
        }
        let Some(n) = cur else { continue };
        let len = text_len(line, at);
        let mut col = 0u16;
        let cols: Vec<u16> = line
            .iter()
            .map(|c| {
                let here = col;
                col += crate::chatwidth::char_w(c.c) as u16;
                here
            })
            .collect();
        for s in spans.iter().filter(|s| s.line + 1 == n) {
            for k in s.start.max(pos)..s.end.min(pos + len) {
                let cell = at + GUTTER_W + (k - pos);
                if let Some(&ix) = cols.get(cell).and_then(|c| index.get(&(i as u16, *c))) {
                    out[ix].deco = Deco {
                        line: DecoLine::Curly,
                        strike: out[ix].deco.strike,
                        color: Some(s.color),
                    };
                }
            }
        }
        pos += len;
    }
}

#[cfg(test)]
#[path = "lspdeco_tests.rs"]
mod tests;

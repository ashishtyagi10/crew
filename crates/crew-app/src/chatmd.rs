//! Maps `md::render`'s output (styled lines wrapped by CHAR count) to card
//! `CardLine`s (styled cells wrapped by DISPLAY column). The two wrap units
//! differ so every produced line is re-chunked here by display width via
//! `chatwidth::fit_end` — the same primitive the old chat-body path used for
//! code chunking — so wide glyphs (CJK, emoji) never overflow the pane.
use std::sync::Arc;

use crate::chatbody::{plain, CardCell, CardLine, Color};
use crate::chatink;
use crate::md::{LineKind, MdLine, MdSpan};

/// Maps one rendered markdown document to card lines, indented one column
/// and re-chunked to `width` display columns per row.
/// One picture the mapped lines reserved room for: where its box starts in
/// the OUTPUT rows, how tall it is, and what to draw there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Picture {
    pub row: usize,
    pub rows: usize,
    pub src: String,
}

pub(crate) fn map_lines(md_lines: Vec<MdLine>, width: usize, fg: Color) -> Vec<CardLine> {
    with_pictures(md_lines, width, fg).0
}

/// [`map_lines`], plus where the pictures ended up.
///
/// The row a picture claims has to be counted HERE and nowhere else: the
/// engine wraps by character and this re-chunks by display column, so a row
/// index taken before the mapping is one a wide glyph can move.
pub(crate) fn with_pictures(
    md_lines: Vec<MdLine>,
    width: usize,
    fg: Color,
) -> (Vec<CardLine>, Vec<Picture>) {
    let mut pics: Vec<Picture> = Vec::new();
    let lines = map_lines_inner(md_lines, width, fg, &mut pics);
    (lines, pics)
}

fn map_lines_inner(
    md_lines: Vec<MdLine>,
    width: usize,
    fg: Color,
    pics: &mut Vec<Picture>,
) -> Vec<CardLine> {
    let muted = crew_theme::theme().text_muted;
    let mut out = Vec::new();
    // Where each fenced block's mapped lines start, so `chatfield` can lay
    // the whole run into one tinted rectangle once its widest line is known.
    let mut runs: Vec<(usize, usize)> = Vec::new();
    let mut block_start: Option<usize> = None;
    for line in md_lines {
        if let LineKind::Picture { i, .. } = line.kind {
            // Every row of the block carries the source; the first one opens
            // the record and the rest extend it, so a picture is one entry
            // however the block was cut.
            let src = crate::md::picture::src_of(&line).unwrap_or_default();
            match pics.last_mut().filter(|_| i > 0) {
                Some(p) => p.rows += 1,
                None => pics.push(Picture {
                    row: out.len(),
                    rows: 1,
                    src: src.to_string(),
                }),
            }
            out.push(vec![plain(' ', fg, false)]);
            continue;
        }
        if line.kind == LineKind::CodeHeader {
            block_start = Some(out.len());
        }
        let line_fg = match line.kind {
            LineKind::CodeHeader | LineKind::CodeFooter | LineKind::Rule => muted,
            LineKind::Quote => chatink::quote_fg(),
            _ => fg,
        };
        let cells: Vec<CardCell> = line
            .spans
            .iter()
            .flat_map(|s| span_cells(s, line.kind, fg, muted))
            .collect();
        push_chunked(&mut out, &cells, width, line_fg);
        if line.kind == LineKind::CodeFooter {
            if let Some(start) = block_start.take() {
                runs.push((start, out.len()));
            }
        }
    }
    // A ```diff fence gets the same word-level marks the viewer's diff
    // rung draws, read back off the ink each line was given. Before the
    // field, which only touches backgrounds — refining reads the ink.
    crate::diffrefine::refine_lines(&mut out);
    crate::chatfield::fill(&mut out, &runs, width);
    out
}

/// Splits `cells` into rows of at most `width` DISPLAY columns (a wide glyph
/// counts two), each prefixed with a one-column indent cell.
fn push_chunked(out: &mut Vec<CardLine>, cells: &[CardCell], width: usize, line_fg: Color) {
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
fn span_cells(span: &MdSpan, kind: LineKind, fg: Color, muted: Color) -> Vec<CardCell> {
    let (cell_fg, bold, italic, bg, link) = span_style(span, kind, fg, muted);
    span.text
        .chars()
        .map(|c| CardCell {
            c,
            fg: cell_fg,
            bold,
            italic,
            bg,
            link: link.clone(),
        })
        .collect()
}

fn span_style(
    span: &MdSpan,
    kind: LineKind,
    fg: Color,
    muted: Color,
) -> (Color, bool, bool, Option<Color>, Option<Arc<str>>) {
    // Checked before `kind`: the quote bar is prefixed to EVERY line of a
    // quote, including the Code lines of a fenced block inside it, and a bar
    // drawn in code colour on a code tint would read as part of the code.
    // A marker carrying a token overrides the marker colour — that is a
    // checked task's ✓, which draws in the diff-added green (`Token::Added`).
    if span.style.marker {
        let fg = match span.style.token {
            crate::md::syntax::Token::Plain => chatink::marker_fg(),
            token => chatink::token_fg(token),
        };
        return (fg, false, false, None, None);
    }
    match kind {
        LineKind::CodeHeader | LineKind::CodeFooter | LineKind::Rule => {
            (muted, false, false, None, None)
        }
        // Inside a fence the SPAN decides the colour, not the line: the
        // tokenizer split it into comment / string / keyword / plain runs at
        // layout time (see `md::layout::code_spans`).
        LineKind::Code => (
            chatink::token_fg(span.style.token),
            // Keywords are marked by weight, not by a colour of their own: a
            // fourth colour would either crowd the ladder the other classes
            // sit on or break the page floor on the darker tubes, and weight
            // works on a single-phosphor screen where hue cannot.
            span.style.token == crate::md::syntax::Token::Keyword,
            false,
            Some(chatink::code_bg()),
            None,
        ),
        // A picture's rows carry only the sentinel span, which is never
        // drawn — the picture itself is paint, laid under these rows.
        LineKind::Blank | LineKind::Picture { .. } => (fg, false, false, None, None),
        LineKind::Body => body_span_style(span, fg),
        LineKind::Quote => body_span_style(span, chatink::quote_fg()),
    }
}

/// Styles one prose span over `base` — the colour its plain text draws in.
/// Precedence, highest first: link, heading, inline code, then `base`. A span
/// can carry several of these at once (`# A [link](u)`), so the order is what
/// decides; it is checked top-down rather than accumulated, so each branch
/// states its whole result.
fn body_span_style(
    span: &MdSpan,
    base: Color,
) -> (Color, bool, bool, Option<Color>, Option<Arc<str>>) {
    let style = span.style;
    // Inline code inside a link keeps the code tint, as it did before.
    let code_bg = if style.code {
        Some(chatink::code_bg())
    } else {
        None
    };
    if let Some(url) = &span.link {
        return (
            chatink::link_color(),
            true,
            style.italic,
            code_bg,
            Some(Arc::from(url.as_str())),
        );
    }
    if style.heading >= 1 {
        return (chatink::heading_fg(), true, style.italic, code_bg, None);
    }
    if style.code {
        return (chatink::code_fg(), style.bold, style.italic, code_bg, None);
    }
    // Prose carrying a token: a checked task item's text, dimmed to the
    // comment rung (`md::tasklist::body_spans`).
    if style.token != crate::md::syntax::Token::Plain {
        let fg = chatink::token_fg(style.token);
        return (fg, style.bold, style.italic, None, None);
    }
    (base, style.bold, style.italic, None, None)
}

#[cfg(test)]
#[path = "chatmd_tests.rs"]
mod tests;

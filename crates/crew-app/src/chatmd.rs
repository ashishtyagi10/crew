//! Maps `md::render`'s output (styled lines wrapped by CHAR count) to card
//! `CardLine`s (styled cells wrapped by DISPLAY column). The two wrap units
//! differ so every produced line is re-chunked here by display width via
//! `chatwidth::fit_end` — the same primitive the old chat-body path used for
//! code chunking — so wide glyphs (CJK, emoji) never overflow the pane.
use std::sync::Arc;

use crate::chatbody::{plain, CardCell, CardLine, Color};
use crate::md::{LineKind, MdLine, MdSpan};

/// The code card's background: the page nudged toward the ink colour, so the
/// block reads as a card in every theme without a dedicated theme slot.
fn code_bg() -> Color {
    let t = crew_theme::theme();
    crate::anim::lerp_rgb(t.page_bg, t.ink, 0.08)
}

/// Link tint: reuse the terminal pane's own URL-highlight colour (`linkhl`)
/// so a link reads the same whether it's in a pane or a chat card.
fn link_color() -> Color {
    crate::linkhl::LINK_FG
}

/// Maps one rendered markdown document to card lines, indented one column
/// and re-chunked to `width` display columns per row.
pub(crate) fn map_lines(md_lines: Vec<MdLine>, width: usize, fg: Color) -> Vec<CardLine> {
    let muted = crew_theme::theme().text_muted;
    let mut out = Vec::new();
    let mut prev_kind: Option<LineKind> = None;
    let mut iter = md_lines.into_iter().peekable();
    while let Some(line) = iter.next() {
        if line.kind == LineKind::Blank {
            // The fenced-code card already draws its own chrome (╭─/╰─) to
            // separate itself from surrounding prose, so the block
            // separator blank the md engine inserts around every top-level
            // block would just be a redundant dead row here — drop it.
            let borders_code = matches!(prev_kind, Some(LineKind::CodeFooter))
                || matches!(iter.peek().map(|l| l.kind), Some(LineKind::CodeHeader));
            if borders_code {
                continue;
            }
        }
        let line_fg = match line.kind {
            LineKind::CodeHeader | LineKind::CodeFooter | LineKind::Rule => muted,
            _ => fg,
        };
        let cells: Vec<CardCell> = line
            .spans
            .iter()
            .flat_map(|s| span_cells(s, line.kind, fg, muted))
            .collect();
        push_chunked(&mut out, &cells, width, line_fg);
        prev_kind = Some(line.kind);
    }
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
    match kind {
        LineKind::CodeHeader | LineKind::CodeFooter | LineKind::Rule => {
            (muted, false, false, None, None)
        }
        LineKind::Code => (fg, false, false, Some(code_bg()), None),
        LineKind::Blank => (fg, false, false, None, None),
        LineKind::Body => {
            let style = span.style;
            let mut cell_fg = fg;
            let mut bold = style.bold;
            match style.heading {
                1 | 2 => {
                    cell_fg = crew_theme::theme().ink;
                    bold = true;
                }
                h if h >= 3 => bold = true,
                _ => {}
            }
            let bg = if style.code { Some(code_bg()) } else { None };
            let mut link = None;
            if let Some(url) = &span.link {
                bold = true;
                cell_fg = link_color();
                link = Some(Arc::from(url.as_str()));
            }
            (cell_fg, bold, style.italic, bg, link)
        }
    }
}

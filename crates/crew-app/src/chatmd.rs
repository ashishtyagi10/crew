//! Maps `md::render`'s output (styled lines wrapped by CHAR count) to card
//! `CardLine`s (styled cells wrapped by DISPLAY column). The two wrap units
//! differ so every produced line is re-chunked here by display width via
//! `chatwidth::fit_end` — the same primitive the old chat-body path used for
//! code chunking — so wide glyphs (CJK, emoji) never overflow the pane.
//! What a span draws in is `chatspan`'s call; this file only places cells.
use std::path::Path;

use crate::chatbody::{plain, CardCell, CardLine, Color};
use crate::chatink;
use crate::chatmdcells::{push_chunked, span_cells};
use crate::md::{LineKind, MdLine};

/// One picture the mapped lines reserved room for: where its box starts in
/// the OUTPUT rows, how tall it is, and what to draw there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Picture {
    pub row: usize,
    pub rows: usize,
    pub src: String,
}

/// Maps one rendered markdown document to CHAT card lines, indented one
/// column and re-chunked to `width` display columns per row. A picture is
/// one named row here (`chatimage`) and a top-level heading is ruled under —
/// the card's own reading of the document, which the viewer does not share.
/// No working directory, so a relative picture is never more than named.
pub(crate) fn map_lines(md_lines: Vec<MdLine>, width: usize, fg: Color) -> Vec<CardLine> {
    map_lines_inner(md_lines, width, fg, None, None)
}

/// [`map_lines`] for a pane working in `cwd`: a picture whose source is a
/// local file the cache holds gets its box (`chatimage::box_lines`) above
/// the named row, one still being read a `loading…` under it.
pub(crate) fn map_chat(
    md_lines: Vec<MdLine>,
    width: usize,
    fg: Color,
    cwd: Option<&Path>,
) -> Vec<CardLine> {
    map_lines_inner(md_lines, width, fg, None, cwd)
}

/// [`map_lines`] for the VIEWER, plus where the pictures ended up.
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
    let lines = map_lines_inner(md_lines, width, fg, Some(&mut pics), None);
    (lines, pics)
}

/// `pics` is where the viewer collects its picture boxes; `None` is the chat
/// card, which marks its boxes into the rows themselves (`chatimage`).
fn map_lines_inner(
    md_lines: Vec<MdLine>,
    width: usize,
    fg: Color,
    mut pics: Option<&mut Vec<Picture>>,
    cwd: Option<&Path>,
) -> Vec<CardLine> {
    let muted = crew_theme::theme().text_muted;
    let chat = pics.is_none();
    let mut out = Vec::new();
    // Where each fenced block's mapped lines start, so `chatfield` can lay
    // the whole run into one tinted rectangle once its widest line is known.
    let mut runs: Vec<(usize, usize)> = Vec::new();
    let mut block_start: Option<usize> = None;
    // Whether the line above was an h1 row: the badge leads a heading's
    // FIRST row only, not each row it wraps onto.
    let mut in_h1 = false;
    let mut lines = md_lines.into_iter().peekable();
    while let Some(line) = lines.next() {
        if let LineKind::Picture { i, .. } = line.kind {
            match pics.as_deref_mut() {
                Some(pics) => reserve(pics, &line, i, out.len()),
                None if i > 0 => continue,
                None => {
                    chat_picture(&mut out, &line, width, fg, muted, cwd);
                    continue;
                }
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
        let level = crate::md::heading::level_of(&line);
        let mut cells: Vec<CardCell> = match line.kind {
            // The language as a badge on the field (`fencebadge`); a
            // quote's bar, prefixed as a marker span, stays a bar.
            LineKind::CodeHeader => line
                .spans
                .iter()
                .flat_map(|s| match s.style.marker {
                    true => span_cells(s, line.kind, fg, muted),
                    false => crate::fencebadge::cells(&s.text),
                })
                .collect(),
            _ => line
                .spans
                .iter()
                .flat_map(|s| span_cells(s, line.kind, fg, muted))
                .collect(),
        };
        if chat && level == 1 && !in_h1 {
            cells.splice(0..0, crate::chatheading::badge_cells(fg));
        }
        in_h1 = level == 1;
        push_chunked(&mut out, &cells, width, line_fg);
        if line.kind == LineKind::CodeFooter {
            if let Some(start) = block_start.take() {
                runs.push((start, out.len()));
            }
        }
        // The rule goes under the LAST row of a wrapped heading, not each.
        let wraps = lines
            .peek()
            .is_some_and(|n| crate::md::heading::level_of(n) == 1);
        if chat && level == 1 && !wraps {
            let rule = crate::chatheading::rule_row(out.last().expect("pushed"), muted);
            out.push(rule);
        }
    }
    // A ```diff fence gets the same word-level marks the viewer's diff
    // rung draws, read back off the ink each line was given. Before the
    // field, which only touches backgrounds — refining reads the ink.
    crate::diffrefine::refine_lines(&mut out);
    crate::chatfield::fill(&mut out, &runs, width);
    out
}

/// A picture on a CHAT card: its box when the cache holds it, the named
/// row — the `[image]` tag is a mark, like a bullet: marker ink — and a
/// `loading…` under that while the worker is still out.
fn chat_picture(
    out: &mut Vec<CardLine>,
    line: &MdLine,
    width: usize,
    fg: Color,
    muted: Color,
    cwd: Option<&Path>,
) {
    use crate::chatimage::{self, Fate};
    let src = crate::md::picture::src_of(line).unwrap_or_default();
    let fate = chatimage::fate(src, cwd);
    if let Fate::Painted(_) = &fate {
        out.extend(chatimage::box_lines(src, width, fg));
    }
    let mark = chatink::marker_fg();
    push_chunked(out, &chatimage::cells(line, mark), width, mark);
    if fate == Fate::Loading {
        push_chunked(out, &chatimage::loading_cells(muted), width, muted);
    }
}

/// Records row `i` of a picture block landing at output row `at`. Every row
/// of the block carries the source; the first one opens the record and the
/// rest extend it, so a picture is one entry however the block was cut.
fn reserve(pics: &mut Vec<Picture>, line: &MdLine, i: u16, at: usize) {
    let src = crate::md::picture::src_of(line).unwrap_or_default();
    match pics.last_mut().filter(|_| i > 0) {
        Some(p) => p.rows += 1,
        None => pics.push(Picture {
            row: at,
            rows: 1,
            src: src.to_string(),
        }),
    }
}

#[cfg(test)]
#[path = "chatmd_tests.rs"]
mod tests;

//! Lays out parsed `Block`s into wrapped, styled `MdLine`s at a fixed column
//! width. Word-wrap/truncation primitives live in `wrap.rs`, table layout in
//! `table.rs` — both split out to keep this file under budget.
use super::parse::{Block, ListItem};
#[cfg(test)]
use super::render;
use super::{LineKind, MdLine, MdSpan};
use wrap::{marker_span, plain_span, split_hardbreaks, wrap_group};

#[path = "table.rs"]
mod table;
#[path = "wrap.rs"]
mod wrap;

/// Reaches `table::lines` for `md::table_lines` (the viewer's CSV rung):
/// `table` is a private submodule of `layout`, not of `md`.
pub(super) fn table_lines(
    header: Vec<Vec<MdSpan>>,
    rows: Vec<Vec<Vec<MdSpan>>>,
    cols: usize,
) -> Vec<MdLine> {
    // A CSV has no delimiter row to declare alignment with; every column
    // reads left, which is what it did before there was a choice.
    table::lines(header, Vec::new(), rows, cols)
}

/// Turns parsed blocks into drawable lines, inserting exactly one
/// `LineKind::Blank` between top-level blocks (none leading/trailing).
/// Footnote definitions come out of the stream and go last (see `footnote`).
pub(super) fn lines(blocks: Vec<Block>, cols: usize) -> Vec<MdLine> {
    let (blocks, notes) = super::footnote::split(blocks);
    let mut out = Vec::new();
    for (i, block) in blocks.into_iter().enumerate() {
        if i > 0 {
            out.push(MdLine {
                spans: Vec::new(),
                kind: LineKind::Blank,
            });
        }
        out.extend(block_lines(block, cols));
    }
    let after_content = !out.is_empty();
    out.extend(super::footnote::lines(notes, cols, after_content));
    out
}

fn block_lines(block: Block, cols: usize) -> Vec<MdLine> {
    match block {
        Block::Paragraph(spans) => match super::picture::lines(&spans) {
            Some(rows) => rows,
            None => wrap_prose_lines(spans, cols),
        },
        Block::Heading(level, spans) => super::heading::lines(level, spans, cols),
        Block::CodeBlock { lang, lines } => super::codeblock::lines(lang, lines, cols),
        Block::List(items) => list_lines(items, cols),
        Block::BlockQuote(inner) => quote_lines(inner, cols),
        Block::Table {
            header,
            aligns,
            rows,
        } => table::lines(header, aligns, rows, cols),
        Block::Rule => vec![MdLine {
            spans: vec![plain_span("─".repeat(cols))],
            kind: LineKind::Rule,
        }],
        // Never reached: `lines` split every definition out before this.
        Block::Footnote { .. } => Vec::new(),
    }
}

pub(super) fn wrap_prose_lines(spans: Vec<MdSpan>, cols: usize) -> Vec<MdLine> {
    split_hardbreaks(spans)
        .into_iter()
        .flat_map(|g| wrap_group(&g, cols))
        .map(|spans| MdLine {
            spans,
            kind: LineKind::Body,
        })
        .collect()
}

fn list_lines(items: Vec<ListItem>, cols: usize) -> Vec<MdLine> {
    let mut out = Vec::new();
    for item in items {
        let indent = "  ".repeat(item.depth as usize);
        let bullet = super::tasklist::bullet(item.task, item.ordered_idx, item.depth);
        let prefix = format!("{indent}{bullet}");
        let prefix_len = prefix.chars().count();
        let avail = cols.saturating_sub(prefix_len).max(1);
        let mut first = true;
        for group in split_hardbreaks(super::tasklist::body_spans(item.spans, item.task)) {
            for line_spans in wrap_group(&group, avail) {
                let mut spans = vec![if first {
                    super::tasklist::head_span(prefix.clone(), item.task)
                } else {
                    plain_span(" ".repeat(prefix_len))
                }];
                first = false;
                spans.extend(line_spans);
                out.push(MdLine {
                    spans,
                    kind: LineKind::Body,
                });
            }
        }
    }
    out
}

fn quote_lines(inner: Vec<Block>, cols: usize) -> Vec<MdLine> {
    let prefix = format!("{} ", crate::glyphs::pick(crate::glyphs::Glyph::Quote)); // ▎
    let prefix_len = prefix.chars().count();
    let inner_cols = cols.saturating_sub(prefix_len).max(1);
    let mut sub = lines(inner, inner_cols);
    for line in sub.iter_mut() {
        if line.kind == LineKind::Blank {
            continue;
        }
        let mut spans = vec![marker_span(prefix.clone())];
        spans.append(&mut line.spans);
        line.spans = spans;
        // ONLY prose becomes Quote. A fenced block inside a quote keeps its
        // Code/CodeHeader/CodeFooter kind so it still renders as a code card,
        // and a rule stays a rule — the bar is prepended to all of them, but
        // the kind is what decides how the line is drawn.
        if line.kind == LineKind::Body {
            line.kind = LineKind::Quote;
        }
    }
    sub
}

#[cfg(test)]
#[path = "layout_tests.rs"]
mod tests;

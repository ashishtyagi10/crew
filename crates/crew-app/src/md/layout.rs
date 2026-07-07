//! Lays out parsed `Block`s into wrapped, styled `MdLine`s at a fixed column
//! width. Word-wrap/truncation primitives live in `wrap.rs`, table layout in
//! `table.rs` — both split out to keep this file under budget.
use super::parse::{Block, ListItem};
#[cfg(test)]
use super::render;
use super::{LineKind, MdLine, MdSpan};
use wrap::{plain_span, split_hardbreaks, wrap_group};

#[path = "table.rs"]
mod table;
#[path = "wrap.rs"]
mod wrap;

/// Turns parsed blocks into drawable lines, inserting exactly one
/// `LineKind::Blank` between top-level blocks (none leading/trailing).
pub(super) fn lines(blocks: Vec<Block>, cols: usize) -> Vec<MdLine> {
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
    out
}

fn block_lines(block: Block, cols: usize) -> Vec<MdLine> {
    match block {
        Block::Paragraph(spans) => wrap_prose_lines(spans, cols),
        Block::Heading(level, mut spans) => {
            for s in spans.iter_mut() {
                s.style.bold = true;
                s.style.heading = level;
            }
            wrap_prose_lines(spans, cols)
        }
        Block::CodeBlock { lang, lines } => code_block_lines(lang, lines, cols),
        Block::List(items) => list_lines(items, cols),
        Block::BlockQuote(inner) => quote_lines(inner, cols),
        Block::Table { header, rows } => table::lines(header, rows, cols),
        Block::Rule => vec![MdLine {
            spans: vec![plain_span("─".repeat(cols))],
            kind: LineKind::Rule,
        }],
    }
}

fn wrap_prose_lines(spans: Vec<MdSpan>, cols: usize) -> Vec<MdLine> {
    split_hardbreaks(spans)
        .into_iter()
        .flat_map(|g| wrap_group(&g, cols))
        .map(|spans| MdLine {
            spans,
            kind: LineKind::Body,
        })
        .collect()
}

fn code_block_lines(lang: String, src_lines: Vec<String>, cols: usize) -> Vec<MdLine> {
    let cw = cols.max(1);
    let lang = if lang.is_empty() { "code" } else { &lang };
    let header_text = format!("╭─ {lang}").chars().take(cw).collect::<String>();
    let mut out = vec![MdLine {
        spans: vec![plain_span(header_text)],
        kind: LineKind::CodeHeader,
    }];
    for line in src_lines {
        let chars: Vec<char> = line.chars().collect();
        if chars.is_empty() {
            out.push(MdLine {
                spans: vec![plain_span(String::new())],
                kind: LineKind::Code,
            });
        } else {
            for chunk in chars.chunks(cw) {
                out.push(MdLine {
                    spans: vec![plain_span(chunk.iter().collect())],
                    kind: LineKind::Code,
                });
            }
        }
    }
    let footer_text = "╰─".chars().take(cw).collect::<String>();
    out.push(MdLine {
        spans: vec![plain_span(footer_text)],
        kind: LineKind::CodeFooter,
    });
    out
}

fn list_lines(items: Vec<ListItem>, cols: usize) -> Vec<MdLine> {
    let mut out = Vec::new();
    for item in items {
        let indent = "  ".repeat(item.depth as usize);
        let bullet = match item.ordered_idx {
            Some(n) => format!("{n}. "),
            None => "• ".to_string(),
        };
        let prefix = format!("{indent}{bullet}");
        let prefix_len = prefix.chars().count();
        let avail = cols.saturating_sub(prefix_len).max(1);
        let mut first = true;
        for group in split_hardbreaks(item.spans) {
            for line_spans in wrap_group(&group, avail) {
                let mut spans = vec![plain_span(if first {
                    prefix.clone()
                } else {
                    " ".repeat(prefix_len)
                })];
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
    const PREFIX: &str = "▎ ";
    let prefix_len = PREFIX.chars().count();
    let inner_cols = cols.saturating_sub(prefix_len).max(1);
    let mut sub = lines(inner, inner_cols);
    for line in sub.iter_mut() {
        if line.kind == LineKind::Blank {
            continue;
        }
        let mut spans = vec![plain_span(PREFIX.to_string())];
        spans.append(&mut line.spans);
        line.spans = spans;
    }
    sub
}

#[cfg(test)]
#[path = "layout_tests.rs"]
mod tests;

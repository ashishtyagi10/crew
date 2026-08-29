//! Lays out parsed `Block`s into wrapped, styled `MdLine`s at a fixed column
//! width. Word-wrap/truncation primitives live in `wrap.rs`, table layout in
//! `table.rs` — both split out to keep this file under budget.
use super::parse::{Block, ListItem};
#[cfg(test)]
use super::render;
use super::MdStyle;
use super::{LineKind, MdLine, MdSpan};
use wrap::{marker_span, plain_span, split_hardbreaks, wrap_group};

#[path = "table.rs"]
mod table;
#[path = "wrap.rs"]
mod wrap;

/// Reaches `table::lines` for `md::table_lines` (the viewer's CSV rung).
/// `table` is a private submodule of `layout`, not of `md` — this is the
/// narrowest crack that exposes it to `md/mod.rs` without loosening
/// `table::lines`'s own `pub(super)` visibility any further than needed.
pub(super) fn table_lines(
    header: Vec<Vec<MdSpan>>,
    rows: Vec<Vec<Vec<MdSpan>>>,
    cols: usize,
) -> Vec<MdLine> {
    table::lines(header, rows, cols)
}

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

/// One wrapped row's worth of spans, taken from `runs` starting at `cursor`
/// (a character offset into the whole line) and advancing it by `width`.
///
/// The cursor lives across rows because a token may straddle a wrap: the two
/// halves become two spans carrying the same token, which is what keeps a long
/// string one colour all the way down.
fn code_spans(
    runs: &[(String, super::syntax::Token)],
    cursor: &mut usize,
    width: usize,
) -> Vec<MdSpan> {
    let (start, end) = (*cursor, *cursor + width);
    *cursor = end;
    let mut out: Vec<MdSpan> = Vec::new();
    let mut at = 0usize;
    for (text, token) in runs {
        let len = text.chars().count();
        let (from, to) = (at.max(start), (at + len).min(end));
        at += len;
        if from >= to {
            continue;
        }
        let slice: String = text
            .chars()
            .skip(from - (at - len))
            .take(to - from)
            .collect();
        out.push(MdSpan {
            text: slice,
            style: MdStyle {
                token: *token,
                ..MdStyle::default()
            },
            link: None,
        });
    }
    if out.is_empty() {
        out.push(plain_span(String::new()));
    }
    out
}

fn code_block_lines(lang: String, src_lines: Vec<String>, cols: usize) -> Vec<MdLine> {
    // An untagged fence whose body reads as a diff is treated as one — the
    // same sniff family as `viewpane::detect::by_content` — so ```diff and a
    // bare paste of `git diff` output colour alike.
    let lang = if lang.is_empty() && super::syntaxdiff::looks_like_diff(&src_lines) {
        "diff".to_string()
    } else {
        lang
    };
    // Two columns narrower than the card: the chat renderer lays these lines
    // into a padded field (`chatfield::PAD` each side), and a code line that
    // used the full width would push its own right-hand pad off the card.
    let cw = cols.saturating_sub(crate::chatfield::PAD * 2).max(1);
    let label = if lang.is_empty() { "code" } else { &lang };
    // The label alone: the block's edges are drawn by the tinted FIELD the
    // chat card lays these lines into (`chatfield`), not by corner glyphs.
    let header_text = crate::chatwidth::clip_w(label, cw);
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
            // Tokenize the WHOLE source line, then cut it into `cw`-wide
            // rows. Doing it the other way round would lex each wrapped chunk
            // independently, and a string or comment that crossed a wrap
            // boundary would change colour mid-token.
            let runs = super::syntax::tokenize(&line, &lang);
            let mut cursor = 0usize;
            for chunk in chars.chunks(cw) {
                out.push(MdLine {
                    spans: code_spans(&runs, &mut cursor, chunk.len()),
                    kind: LineKind::Code,
                });
            }
        }
    }
    // Closed by a blank row of the field rather than a corner — see the
    // header above.
    out.push(MdLine {
        spans: vec![plain_span(String::new())],
        kind: LineKind::CodeFooter,
    });
    out
}

fn list_lines(items: Vec<ListItem>, cols: usize) -> Vec<MdLine> {
    let mut out = Vec::new();
    for item in items {
        let indent = "  ".repeat(item.depth as usize);
        let bullet = super::tasklist::bullet(item.task, item.ordered_idx);
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
    const PREFIX: &str = "▎ ";
    let prefix_len = PREFIX.chars().count();
    let inner_cols = cols.saturating_sub(prefix_len).max(1);
    let mut sub = lines(inner, inner_cols);
    for line in sub.iter_mut() {
        if line.kind == LineKind::Blank {
            continue;
        }
        let mut spans = vec![marker_span(PREFIX.to_string())];
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

//! Folds a stream of pulldown-cmark events into a tree of `Block`s. Inline
//! styling lives in `inline.rs`; bare-URL detection lives in `autolink.rs`.
//! `md::layout` is the consumer.
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use super::MdSpan;

#[path = "autolink.rs"]
mod autolink;
#[path = "fold.rs"]
mod fold;
#[path = "inline.rs"]
mod inline;

use fold::{collect_code_block, collect_html_block, collect_list_items, collect_table};
use inline::collect_inline;

/// Nesting cap for `BlockQuote`/`List`: past this depth, further nesting is
/// folded flat instead of recursed into, so pathological input (e.g. 50k
/// `>` in a row) can't blow the call stack rendering untrusted text.
const MAX_NEST_DEPTH: u8 = 32;

/// One list entry: `ordered_idx` is `Some(n)` for the nth item of an ordered
/// list, `None` for bullet items; `depth` is 0 at the list's own level and
/// increments for each level of nesting.
#[derive(Debug, PartialEq)]
pub(super) struct ListItem {
    pub ordered_idx: Option<u64>,
    pub depth: u8,
    pub spans: Vec<MdSpan>,
}

/// A parsed markdown block. `Block::List` flattens nested lists into one
/// vector, distinguished by `ListItem::depth`.
// CodeBlock/BlockQuote are markdown domain terms, not name repetition.
#[allow(clippy::enum_variant_names)]
#[derive(Debug, PartialEq)]
pub(super) enum Block {
    Paragraph(Vec<MdSpan>),
    Heading(u8, Vec<MdSpan>),
    CodeBlock {
        lang: String,
        lines: Vec<String>,
    },
    List(Vec<ListItem>),
    BlockQuote(Vec<Block>),
    Table {
        header: Vec<Vec<MdSpan>>,
        rows: Vec<Vec<Vec<MdSpan>>>,
    },
    Rule,
}

/// Parses `text` into blocks. Never panics. CommonMark: a soft break joins with a space.
pub(super) fn parse(text: &str) -> Vec<Block> {
    parse_with(text, false)
}

/// Same, but `keep_soft_breaks` keeps each line break as its own line — chat prose (`md::render_chat`).
pub(super) fn parse_with(text: &str, keep_soft_breaks: bool) -> Vec<Block> {
    let opts = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH;
    let mut events = Parser::new_ext(text, opts);
    collect_blocks(&mut events, 0, keep_soft_breaks)
}

/// Also used recursively for `BlockQuote` contents: a nested quote's own
/// `Start`/`End` pair is fully consumed by its own call, so the first stray
/// `End(BlockQuote)` seen is this call's own close. Past `MAX_NEST_DEPTH`,
/// further quotes fold flat instead of recursing (`inert` tracks the extras).
fn collect_blocks<'a>(
    events: &mut impl Iterator<Item = Event<'a>>,
    depth: u8,
    keep_soft_breaks: bool,
) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut inert = 0u32;
    while let Some(event) = events.next() {
        match event {
            Event::Start(Tag::Paragraph) => {
                let spans = collect_inline(events, TagEnd::Paragraph, keep_soft_breaks);
                blocks.push(Block::Paragraph(spans));
            }
            Event::Start(Tag::Heading { level, .. }) => {
                let inline = collect_inline(events, TagEnd::Heading(level), keep_soft_breaks);
                blocks.push(Block::Heading(level as u8, heading_spans(inline, level)));
            }
            Event::Start(Tag::CodeBlock(kind)) => blocks.push(collect_code_block(events, kind)),
            Event::Start(Tag::List(start)) => {
                let (items, hoisted) = collect_list_items(events, start, 0, keep_soft_breaks);
                blocks.push(Block::List(items));
                // Fenced code found inside a list item can't live in a
                // `ListItem`'s flat spans, so `collect_list_items` hoists it
                // out; it renders as a sibling code block after the list.
                blocks.extend(hoisted);
            }
            Event::Start(Tag::BlockQuote(_)) if depth < MAX_NEST_DEPTH => {
                let inner = collect_blocks(events, depth + 1, keep_soft_breaks);
                blocks.push(Block::BlockQuote(inner));
            }
            Event::Start(Tag::BlockQuote(_)) => inert += 1,
            Event::End(TagEnd::BlockQuote(_)) if inert > 0 => inert -= 1,
            Event::End(TagEnd::BlockQuote(_)) => break,
            Event::Start(Tag::Table(_)) => blocks.push(collect_table(events, keep_soft_breaks)),
            Event::Start(Tag::HtmlBlock) => blocks.push(collect_html_block(events)),
            Event::Rule => blocks.push(Block::Rule),
            _ => {} // stray leaf/garbage events at block level: ignore
        }
    }
    blocks
}

fn heading_spans(spans: Vec<MdSpan>, level: HeadingLevel) -> Vec<MdSpan> {
    let level = level as u8;
    spans
        .into_iter()
        .map(|mut s| {
            s.style.heading = level;
            s
        })
        .collect()
}

#[cfg(test)]
#[path = "parse_tests.rs"]
mod tests;

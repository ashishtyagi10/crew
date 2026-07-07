//! Folds a stream of pulldown-cmark events into a tree of `Block`s. Inline
//! styling lives in `inline.rs`; bare-URL detection lives in `autolink.rs`.
//!
//! Nothing outside `#[cfg(test)]` calls `parse` yet — `md::layout` (Task 2)
//! is the real consumer, so the whole tree reads as dead code to a release
//! build until then.
#![allow(dead_code)]
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use super::MdSpan;

#[path = "autolink.rs"]
mod autolink;
#[path = "fold.rs"]
mod fold;
#[path = "inline.rs"]
mod inline;

use fold::collect_table;
use inline::{apply_inline_event, collect_inline, fold_nested_list, InlineState};

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

/// Parses `text` into a flat sequence of top-level blocks. Never panics.
pub(super) fn parse(text: &str) -> Vec<Block> {
    let opts = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH;
    let mut events = Parser::new_ext(text, opts);
    collect_blocks(&mut events, 0)
}

/// Also used recursively for `BlockQuote` contents: a nested quote's own
/// `Start`/`End` pair is fully consumed by its own recursive call, so the
/// first stray `End(BlockQuote)` a call sees is its own closing tag. Once
/// `depth` hits `MAX_NEST_DEPTH`, further quotes stop recursing: `inert`
/// counts those extra opens so their closes don't end this call early, and
/// their contents fold into `blocks` at the current level instead.
fn collect_blocks<'a>(events: &mut impl Iterator<Item = Event<'a>>, depth: u8) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut inert = 0u32;
    while let Some(event) = events.next() {
        match event {
            Event::Start(Tag::Paragraph) => {
                blocks.push(Block::Paragraph(collect_inline(events, TagEnd::Paragraph)))
            }
            Event::Start(Tag::Heading { level, .. }) => {
                let spans = heading_spans(collect_inline(events, TagEnd::Heading(level)), level);
                blocks.push(Block::Heading(level as u8, spans));
            }
            Event::Start(Tag::CodeBlock(kind)) => blocks.push(collect_code_block(events, kind)),
            Event::Start(Tag::List(start)) => {
                blocks.push(Block::List(collect_list_items(events, start, 0)))
            }
            Event::Start(Tag::BlockQuote(_)) if depth < MAX_NEST_DEPTH => {
                blocks.push(Block::BlockQuote(collect_blocks(events, depth + 1)))
            }
            Event::Start(Tag::BlockQuote(_)) => inert += 1,
            Event::End(TagEnd::BlockQuote(_)) if inert > 0 => inert -= 1,
            Event::End(TagEnd::BlockQuote(_)) => break,
            Event::Start(Tag::Table(_)) => blocks.push(collect_table(events)),
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

fn collect_code_block<'a>(
    events: &mut impl Iterator<Item = Event<'a>>,
    kind: CodeBlockKind,
) -> Block {
    let lang = match kind {
        CodeBlockKind::Fenced(l) => l.into_string(),
        CodeBlockKind::Indented => String::new(),
    };
    let mut buf = String::new();
    loop {
        match events.next() {
            Some(Event::Text(t)) => buf.push_str(&t),
            Some(Event::End(TagEnd::CodeBlock)) | None => break,
            _ => {}
        }
    }
    let mut lines: Vec<String> = buf.split('\n').map(String::from).collect();
    if lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    Block::CodeBlock { lang, lines }
}

fn collect_list_items<'a>(
    events: &mut impl Iterator<Item = Event<'a>>,
    start: Option<u64>,
    depth: u8,
) -> Vec<ListItem> {
    let mut items = Vec::new();
    let mut idx = start;
    loop {
        match events.next() {
            Some(Event::Start(Tag::Item)) => {
                let (spans, nested) = collect_item(events, depth);
                items.push(ListItem {
                    ordered_idx: idx,
                    depth,
                    spans,
                });
                idx = idx.map(|n| n + 1);
                items.extend(nested);
            }
            Some(Event::End(TagEnd::List(_))) | None => break,
            _ => {}
        }
    }
    items
}

/// An item's own inline spans plus any nested list found inside it (nested
/// lists are hoisted out and appended after the item by the caller).
fn collect_item<'a>(
    events: &mut impl Iterator<Item = Event<'a>>,
    depth: u8,
) -> (Vec<MdSpan>, Vec<ListItem>) {
    let mut spans = Vec::new();
    let mut nested = Vec::new();
    let mut state = InlineState::default();
    loop {
        match events.next() {
            Some(Event::End(TagEnd::Item)) | None => break,
            Some(Event::Start(Tag::List(start))) if depth < MAX_NEST_DEPTH => {
                nested = collect_list_items(events, start, depth + 1)
            }
            Some(Event::Start(Tag::List(_))) => fold_nested_list(events, &mut spans),
            Some(Event::Start(Tag::Paragraph)) => {
                spans.extend(collect_inline(events, TagEnd::Paragraph))
            }
            Some(event) => apply_inline_event(event, &mut state, &mut spans),
        }
    }
    (autolink::autolink(spans), nested)
}

#[cfg(test)]
#[path = "parse_tests.rs"]
mod tests;

//! Block-collection helpers folded out of `parse.rs` to keep that file under
//! its line budget: table folding, fenced-code-block folding, and list-item
//! folding (including hoisting fenced code found inside a list item out to a
//! real sibling `Block::CodeBlock`, since a list item's flat `MdSpan`s have
//! nowhere to hold one).
use pulldown_cmark::{CodeBlockKind, Event, Tag, TagEnd};

use super::inline::{apply_inline_event, collect_inline, fold_nested_list, InlineState};
use super::{autolink, Block, ListItem, MAX_NEST_DEPTH};
use crate::md::{MdSpan, MdStyle};

/// A never-rendered marker span (`char_w('\n') == 0`) that `wrap::split_hardbreaks`
/// splits on to force a line break — the same convention `inline::apply_inline_event`
/// uses for soft/hard breaks.
pub(super) fn newline_marker() -> MdSpan {
    MdSpan {
        text: "\n".into(),
        style: MdStyle::default(),
        link: None,
    }
}

pub(super) fn collect_table<'a>(
    events: &mut impl Iterator<Item = Event<'a>>,
    keep_soft_breaks: bool,
) -> Block {
    let mut header = Vec::new();
    let mut rows = Vec::new();
    loop {
        match events.next() {
            Some(Event::Start(Tag::TableHead)) => {
                header = collect_row(events, TagEnd::TableHead, keep_soft_breaks)
            }
            Some(Event::Start(Tag::TableRow)) => {
                rows.push(collect_row(events, TagEnd::TableRow, keep_soft_breaks))
            }
            Some(Event::End(TagEnd::Table)) | None => break,
            _ => {}
        }
    }
    Block::Table { header, rows }
}

fn collect_row<'a>(
    events: &mut impl Iterator<Item = Event<'a>>,
    stop: TagEnd,
    keep_soft_breaks: bool,
) -> Vec<Vec<MdSpan>> {
    let mut cells = Vec::new();
    loop {
        match events.next() {
            Some(Event::Start(Tag::TableCell)) => {
                cells.push(collect_inline(events, TagEnd::TableCell, keep_soft_breaks))
            }
            Some(Event::End(end)) if end == stop => break,
            None => break,
            _ => {}
        }
    }
    cells
}

pub(super) fn collect_code_block<'a>(
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

/// Collects one list's items. Returns the flattened items (nested sub-lists
/// folded in by depth, as before) plus any fenced code blocks found inside
/// an item — hoisted out here because `ListItem::spans` is flat `MdSpan`s
/// with no room for a real code block. Hoisted blocks land as siblings after
/// the whole list (not interleaved at the exact item) — simpler, and no
/// caller needs finer placement today.
pub(super) fn collect_list_items<'a>(
    events: &mut impl Iterator<Item = Event<'a>>,
    start: Option<u64>,
    depth: u8,
    keep_soft_breaks: bool,
) -> (Vec<ListItem>, Vec<Block>) {
    let mut items = Vec::new();
    let mut hoisted = Vec::new();
    let mut idx = start;
    loop {
        match events.next() {
            Some(Event::Start(Tag::Item)) => {
                let (spans, nested, item_hoisted) = collect_item(events, depth, keep_soft_breaks);
                items.push(ListItem {
                    ordered_idx: idx,
                    depth,
                    spans,
                });
                idx = idx.map(|n| n + 1);
                items.extend(nested);
                hoisted.extend(item_hoisted);
            }
            Some(Event::End(TagEnd::List(_))) | None => break,
            _ => {}
        }
    }
    (items, hoisted)
}

/// An item's own inline spans, any nested list found inside it (hoisted out
/// and appended after the item by the caller), and any fenced code blocks
/// found inside it (hoisted out as sibling `Block::CodeBlock`s). Successive
/// paragraphs within one item (`- a\n\n  b`) get a `newline_marker` between
/// them so they don't silently concatenate into one run of text.
fn collect_item<'a>(
    events: &mut impl Iterator<Item = Event<'a>>,
    depth: u8,
    keep_soft_breaks: bool,
) -> (Vec<MdSpan>, Vec<ListItem>, Vec<Block>) {
    let mut spans = Vec::new();
    let mut nested = Vec::new();
    let mut hoisted = Vec::new();
    let mut state = InlineState::new(keep_soft_breaks);
    loop {
        match events.next() {
            Some(Event::End(TagEnd::Item)) | None => break,
            Some(Event::Start(Tag::List(start))) if depth < MAX_NEST_DEPTH => {
                let (n, h) = collect_list_items(events, start, depth + 1, keep_soft_breaks);
                nested = n;
                hoisted.extend(h);
            }
            Some(Event::Start(Tag::List(_))) => {
                fold_nested_list(events, &mut spans, keep_soft_breaks)
            }
            Some(Event::Start(Tag::CodeBlock(kind))) => {
                hoisted.push(collect_code_block(events, kind))
            }
            Some(Event::Start(Tag::Paragraph)) => {
                if !spans.is_empty() {
                    spans.push(newline_marker());
                }
                spans.extend(collect_inline(events, TagEnd::Paragraph, keep_soft_breaks))
            }
            Some(event) => apply_inline_event(event, &mut state, &mut spans),
        }
    }
    (autolink::autolink(spans), nested, hoisted)
}

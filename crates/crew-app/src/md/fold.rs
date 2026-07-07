//! Table folding: turns pulldown-cmark table events into `Block::Table`.
//! Split out of `parse.rs` to keep that file under its line budget.
use pulldown_cmark::{Event, Tag, TagEnd};

use super::inline::collect_inline;
use super::Block;
use crate::md::MdSpan;

pub(super) fn collect_table<'a>(events: &mut impl Iterator<Item = Event<'a>>) -> Block {
    let mut header = Vec::new();
    let mut rows = Vec::new();
    loop {
        match events.next() {
            Some(Event::Start(Tag::TableHead)) => header = collect_row(events, TagEnd::TableHead),
            Some(Event::Start(Tag::TableRow)) => rows.push(collect_row(events, TagEnd::TableRow)),
            Some(Event::End(TagEnd::Table)) | None => break,
            _ => {}
        }
    }
    Block::Table { header, rows }
}

fn collect_row<'a>(events: &mut impl Iterator<Item = Event<'a>>, stop: TagEnd) -> Vec<Vec<MdSpan>> {
    let mut cells = Vec::new();
    loop {
        match events.next() {
            Some(Event::Start(Tag::TableCell)) => {
                cells.push(collect_inline(events, TagEnd::TableCell))
            }
            Some(Event::End(end)) if end == stop => break,
            None => break,
            _ => {}
        }
    }
    cells
}

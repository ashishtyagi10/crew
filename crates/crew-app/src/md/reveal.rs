//! The block the caret is in, shown as the markdown it is written in.
//!
//! The render hides every marker — that is what rendering means — and an editor that can only
//! ever stand *between* markers cannot remove a heading's `#`, turn a `-` into a `1.`, or fix a
//! link's URL. So the block holding the caret is drawn as its SOURCE lines instead, the way
//! Obsidian's live preview does: its `#`, `**`, `> ` and `[text](url)` in marker ink, its words
//! still bold or still a heading, and every other block rendered as before. Move the caret out
//! and the block renders again.
//!
//! Two halves, both pure:
//!
//! * [`block_at`] answers which source lines to reveal for a byte — the innermost leaf block
//!   holding it, widened to whole lines so the line's leading markers (`# `, `- `, `> `, the
//!   indent) come with it. It is read off the parser, never off the layout.
//! * [`apply`] swaps that block's rendered rows for its source lines. Every character of a
//!   source line IS its byte, so provenance is exact by construction, and every byte of the
//!   block becomes somewhere the caret can stand.
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

/// What kind of block is revealed — a fence is drawn on its code field, fences and all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Prose,
    Fence,
    Indented,
}

/// Source lines to show as written: `[from, to]` in bytes, `to` being the end of the last line
/// (its newline excluded), so a caret at the very end of the block is still inside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Reveal {
    pub from: u32,
    pub to: u32,
    pub kind: Kind,
}

impl Reveal {
    pub(crate) fn holds(&self, at: u32) -> bool {
        (self.from..=self.to).contains(&at)
    }
}

/// The block to reveal for a caret on byte `at` of `src`, if `at` is in one: the innermost of
/// [`blocks`] holding it — a loose item's paragraph rather than the whole item. The blocks are
/// parsed once per version of the text ([`super::memo`]), not once per crossing.
pub(crate) fn block_at(src: &str, at: u32) -> Option<Reveal> {
    let all = super::memo::blocks(src);
    all.iter()
        .filter(|b| b.holds(at))
        .min_by_key(|b| b.to - b.from)
        .copied()
}

/// Every block a caret could reveal in `src`, widened to whole lines.
pub(super) fn blocks(src: &str) -> Vec<Reveal> {
    let opts = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES;
    let mut found: Vec<Reveal> = Vec::new();
    // Open list items, by index into `found`: a tight item has no paragraph of its own, so the
    // ITEM is the leaf — up to where a list or a fence nested inside it starts.
    let mut items: Vec<usize> = Vec::new();
    for (e, r) in Parser::new_ext(src, opts).into_offset_iter() {
        let block = |kind| Reveal {
            from: r.start as u32,
            to: r.end as u32,
            kind,
        };
        // A nested list or fence is its own block (a fence is drawn after the list, out of
        // order), so the item it sits in ends at the newline before it — not at its marker,
        // which sits after an indent, and widening from there would take the line.
        if let (Event::Start(Tag::List(_) | Tag::CodeBlock(_)), Some(&i)) = (&e, items.last()) {
            let cut = src[..r.start].rfind('\n').unwrap_or(r.start);
            found[i].to = found[i].to.min(cut as u32);
        }
        match e {
            Event::Start(Tag::Paragraph | Tag::Heading { .. } | Tag::Table(_) | Tag::HtmlBlock)
            | Event::Rule => found.push(block(Kind::Prose)),
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(_))) => {
                found.push(block(Kind::Fence))
            }
            Event::Start(Tag::CodeBlock(CodeBlockKind::Indented)) => {
                found.push(block(Kind::Indented))
            }
            Event::Start(Tag::Item) => {
                items.push(found.len());
                found.push(block(Kind::Prose));
            }
            Event::End(TagEnd::Item) => {
                items.pop();
            }
            _ => {}
        }
    }
    found.into_iter().map(|b| widen(src, b)).collect()
}

/// Out to whole lines: back to the start of the first, on to the end of the last.
fn widen(src: &str, b: Reveal) -> Reveal {
    let (s, mut e) = (b.from as usize, (b.to as usize).min(src.len()));
    while e > s && src.as_bytes()[e - 1] == b'\n' {
        e -= 1;
    }
    let from = src[..s].rfind('\n').map_or(0, |i| i + 1);
    let to = src[e..].find('\n').map_or(src.len(), |i| e + i);
    Reveal {
        from: from as u32,
        to: to as u32,
        ..b
    }
}

pub(crate) use super::revealrows::apply;

#[cfg(test)]
#[path = "reveal_tests.rs"]
pub(super) mod tests;

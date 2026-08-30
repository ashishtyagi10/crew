//! Where in the file each rendered character came from.
//!
//! This is the field the whole markdown *editor* rests on. A render with no
//! provenance can be read and never edited: put a cursor on the word `window`
//! in a heading and there is no way back to the byte that word starts at, so
//! nothing can be typed, and nothing can be saved without re-serializing the
//! entire document and rewriting every line the author wrote by hand.
//!
//! pulldown-cmark hands a source range to every event through
//! `into_offset_iter()`. Threading `(Event, Range)` through the nine folding
//! functions would change every one of their signatures and every call site,
//! for a value only [`crate::md::inline`] reads — so the ranges ride
//! alongside instead: [`ranged`] wraps the parser, stamping each event's range
//! here as it hands the event on, and the fold reads [`here`] while applying
//! the event it was just given. Every loop in the fold pulls one event and
//! dispatches it immediately, which is what makes that sound.
//!
//! **A span carries provenance only when its text IS its source bytes.**
//! Markdown is not always a verbatim copy of itself: `&amp;` renders as one
//! character from five bytes, `\*` as one from two, a soft break as a space
//! from a newline. Inside such a run, char *n* of the render is not byte *n*
//! of the file and no arithmetic recovers it — so those spans carry nothing,
//! and a cursor simply has no position inside them. Claiming an offset that
//! is off by four is far worse than admitting there is none: one is a cursor
//! that cannot go somewhere, the other writes a character into the middle of
//! an entity.
use std::cell::Cell;

use pulldown_cmark::Event;

thread_local! {
    /// The source range of the event currently being folded, as
    /// `(start, end)` byte offsets.
    static AT: Cell<(u32, u32)> = const { Cell::new((0, 0)) };
}

/// Wrap an offset iterator so it yields plain events, stamping each one's
/// range for [`here`] on the way past.
pub(super) fn ranged<'a>(
    events: impl Iterator<Item = (Event<'a>, std::ops::Range<usize>)>,
) -> impl Iterator<Item = Event<'a>> {
    events.map(|(e, r)| {
        AT.with(|at| at.set((r.start as u32, r.end as u32)));
        e
    })
}

/// The source range of the event being folded right now.
pub(super) fn here() -> (u32, u32) {
    AT.with(|at| at.get())
}

/// The offset to record for a run of `text` produced by the current event, or
/// `None` when the text is not a verbatim copy of its source bytes (see the
/// module comment).
///
/// The length test is necessary and not sufficient — a soft break renders as
/// a space from a newline, one byte for one byte — so the events that do not
/// copy their source do not come through here at all (see `inline`).
pub(super) fn offset_for(text: &str) -> Option<u32> {
    let (start, end) = here();
    (end.saturating_sub(start) as usize == text.len()).then_some(start)
}

/// The offset of the TEXT inside an inline code span, whose event range
/// covers its delimiters. Only the plain `` `code` `` shape is answered: two
/// backticks, no padding spaces, which is the one whose arithmetic is exact.
/// Every other spelling claims nothing rather than claiming a byte one out.
pub(super) fn offset_for_code(text: &str) -> Option<u32> {
    let (start, end) = here();
    (end.saturating_sub(start) as usize == text.len() + 2).then_some(start + 1)
}

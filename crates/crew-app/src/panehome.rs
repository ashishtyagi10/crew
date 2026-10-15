//! Where a pane's buffer is at HOME, for the count its top border stamps
//! (see [`crate::panescroll`]).
//!
//! A shell and a transcript live at their bottom — the newest line lands
//! there — so `⇡N` says how far back you have gone. A file is read from its
//! top. A viewer opened on line 1 is at home, and it wore `⇡110` there:
//! "110 rows up from the end", a scroll you had never taken, stamped on every
//! file you opened. A file counts DOWN from its first line instead, `⇣N`, and
//! says nothing at the top. The count is a window that has left home; where
//! home is depends on what the window is onto.
use crate::pane::PaneContent;

/// What a card frames, as its scroll readings see it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Doc {
    /// Not a document: the gutter is a scrollback affordance, drawn only
    /// once scrolled back from the live bottom.
    #[default]
    No,
    /// A transcript: a document — its gutter says where you ARE, drawn
    /// whenever it is longer than the pane — whose home is the bottom.
    Transcript,
    /// A file, read from its top.
    File,
}

/// The kind of document `content` is.
pub(crate) fn of(content: &PaneContent) -> Doc {
    match content {
        PaneContent::View(_) => Doc::File,
        PaneContent::Chat(_) => Doc::Transcript,
        _ => Doc::No,
    }
}

/// How far the window is from home, and the arrow that points that way:
/// `visible` rows of a `total`-row buffer, `scroll` rows back from its
/// bottom (what every pane reports — see `Bar::scroll`).
pub(crate) fn away(doc: Doc, total: usize, visible: usize, scroll: usize) -> (usize, char) {
    match doc {
        Doc::File => (
            total.saturating_sub(visible).saturating_sub(scroll),
            '\u{21e3}',
        ),
        Doc::No | Doc::Transcript => (scroll, '\u{21e1}'),
    }
}

#[cfg(test)]
#[path = "panehome_tests.rs"]
mod tests;

//! Where each top-level block of a document starts — the seams [`super::blockmemo`] cuts the
//! document at — gathered during the one walk [`super::reveal::walk`] already makes.
use pulldown_cmark::{Event, Tag};

/// The seams seen so far, and whether any block reaches into another.
pub(super) struct Seams<'a> {
    src: &'a str,
    depth: u32,
    starts: Vec<u32>,
    cross: bool,
}

impl<'a> Seams<'a> {
    pub(super) fn new(src: &'a str) -> Self {
        Self {
            src,
            depth: 0,
            starts: Vec::new(),
            cross: false,
        }
    }

    /// One event of the walk, starting at byte `at`.
    pub(super) fn see(&mut self, e: &Event, at: usize) {
        if self.depth == 0 && !matches!(e, Event::End(_)) {
            // Whole lines: an indented code block cut after its indent reparses as a paragraph.
            let line = self.src[..at].rfind('\n').map_or(0, |i| i + 1);
            self.starts.push(line as u32);
        }
        match e {
            // A definition is drawn at the END of the document, and a reference is a mark
            // only while some block defines it: either way one block's render rests on another.
            Event::Start(Tag::FootnoteDefinition(_)) => {
                self.cross = true;
                self.depth += 1;
            }
            Event::FootnoteReference(_) => self.cross = true,
            Event::Start(_) => self.depth += 1,
            Event::End(_) => self.depth = self.depth.saturating_sub(1),
            _ => {}
        }
    }

    /// The chunk starts — the first at 0, so text before the first block is in a chunk too —
    /// or `None` when the blocks cannot be rendered apart: a footnote, a link reference
    /// definition (`refdefs`, which any block's `[text][label]` may resolve against), or two
    /// blocks starting on one line.
    pub(super) fn done(mut self, refdefs: bool) -> Option<Vec<u32>> {
        if self.cross || refdefs || self.starts.windows(2).any(|w| w[0] >= w[1]) {
            return None;
        }
        if let Some(first) = self.starts.first_mut() {
            *first = 0;
        }
        Some(self.starts)
    }
}

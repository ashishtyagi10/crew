//! Which block the caret's markers are shown for.
//!
//! Asked on every `lines_for`, which is called many times a frame — so the parse that answers
//! it runs only when the caret has left the block already revealed. Inside one block every byte
//! gives the same answer: two blocks never share a source line, and the revealed range is whole
//! lines.
use crate::md::reveal::{self, Reveal};
use crate::viewpane::detect::Format;
use crate::viewpane::{LoadState, ViewPane};

impl ViewPane {
    /// The block to draw as its source: the one the caret is in, when this is a markdown
    /// document being edited and rendered.
    pub(crate) fn wanted_reveal(&self) -> Option<Reveal> {
        let at = self.caret.and(self.caret_at)?;
        let LoadState::Ready {
            format: Format::Markdown,
            loaded,
        } = &self.state
        else {
            return None;
        };
        if self.raw {
            return None;
        }
        if let Some(c) = self.cache.borrow().as_ref() {
            // Asked for this very byte already — the answer may be "no block", which has no
            // range to test the fast path below against, and must not re-parse every frame.
            if c.reveal_for == Some(at) {
                return c.reveal;
            }
            if let Some(r) = c.reveal.filter(|r| r.holds(at)) {
                return Some(r);
            }
        }
        let r = reveal::block_at(&loaded.text, at);
        // Remember it when it is what the cache already draws. A caller holding the cache
        // borrowed just goes without the memo.
        if let Ok(mut c) = self.cache.try_borrow_mut() {
            if let Some(c) = c.as_mut().filter(|c| c.reveal == r) {
                c.reveal_for = Some(at);
            }
        }
        r
    }

    /// This pane's lines with `reveal` drawn as its source — `lines_for`'s render, kept here so
    /// the cache above it stays one screen long.
    pub(super) fn rung_lines(
        &self,
        cols: usize,
        invisibles: bool,
        reveal: Option<Reveal>,
    ) -> (
        Vec<crate::chatbody::CardLine>,
        Vec<crate::viewpane::outline::Mark>,
        Vec<crate::chatmd::Picture>,
    ) {
        let (raw, split) = (self.raw, self.split);
        super::lines::for_state_revealed(&self.state, raw, cols, invisibles, split, reveal)
    }
}

#[cfg(test)]
#[path = "caretreveal_tests.rs"]
mod tests;

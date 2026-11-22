//! The document-wide work an editing caret repeats, kept per version of the text.
//!
//! A caret crossing from one block to the next changes which block shows its markers and
//! nothing else, yet each crossing parsed the whole document to find the block (~1.2 ms on the
//! 327 KB manual, release) and rendered the whole document again to splice it in (~6 ms) — on
//! the frame thread. Both answers depend only on the text (and, for the render, the width), so
//! each is kept for the last text seen, keyed by its hash: an edit changes the hash and the
//! next read redoes the work once.
use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use super::reveal::Reveal;
use super::MdLine;

thread_local! {
    static BLOCKS: RefCell<Option<(u64, Rc<Vec<Reveal>>)>> = const { RefCell::new(None) };
    static RENDER: RefCell<Option<(u64, usize, Rc<Vec<MdLine>>)>> = const { RefCell::new(None) };
}

/// The text's identity: its hash, length folded in.
fn key(text: &str) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut h);
    h.finish()
}

/// [`super::reveal::blocks`] of `text`, parsed once per version.
pub(super) fn blocks(text: &str) -> Rc<Vec<Reveal>> {
    let k = key(text);
    BLOCKS.with(|c| {
        let mut c = c.borrow_mut();
        match c.as_ref() {
            Some((have, b)) if *have == k => Rc::clone(b),
            _ => {
                let b = Rc::new(super::reveal::blocks(text));
                *c = Some((k, Rc::clone(&b)));
                b
            }
        }
    })
}

/// [`super::render`] of `text` at `cols`, rendered once per version and width.
pub(super) fn render(text: &str, cols: usize) -> Rc<Vec<MdLine>> {
    let k = key(text);
    RENDER.with(|c| {
        let mut c = c.borrow_mut();
        match c.as_ref() {
            Some((have, w, r)) if *have == k && *w == cols => Rc::clone(r),
            _ => {
                let r = Rc::new(super::render(text, cols));
                *c = Some((k, cols, Rc::clone(&r)));
                r
            }
        }
    })
}

#[cfg(test)]
#[path = "memo_tests.rs"]
mod tests;

//! A keystroke re-renders the block it lands in, not the document.
//!
//! An edit changes one block's source and leaves every other block's exactly as it was, yet
//! [`super::memo`] keys the whole render on the whole text, so each keystroke in the 327 KB
//! manual re-rendered all of it (~16 ms of the ~28 ms a keystroke cost, release, on the frame
//! thread; now ~0.9 ms). Here the document is
//! cut at its top-level blocks' first lines ([`super::blockseam`]) and each piece's render is
//! kept under its own source text, width and icon set: a keystroke renders the one piece it
//! changed, and every other piece is copied from the last render with its byte offsets moved.
//!
//! A piece rendered on its own is the piece rendered in the document only while no block
//! reaches into another — so a document with a footnote or a link reference definition is
//! rendered whole, as before ([`super::reveal::Walk::tops`]). Everything else a block draws —
//! an ordered list's numbers, a quote's bar, a fence hoisted out of a list — comes from its own
//! source.
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use super::{LineKind, MdLine};

/// One piece's render: its blocks' lines, with byte offsets counted from the piece's start.
type Piece = Rc<Vec<Vec<MdLine>>>;

#[derive(Default)]
struct Cache {
    cols: usize,
    nerd: bool,
    pieces: HashMap<String, Piece>,
}

thread_local! {
    static CACHE: RefCell<Cache> = RefCell::new(Cache::default());
    /// Pieces rendered afresh — how a test sees that a keystroke rendered one.
    static RENDERED: Cell<u64> = const { Cell::new(0) };
}

/// [`super::render`] of `text` at `cols` — equal to it line for line — re-rendering only the
/// pieces whose source changed since the last call.
pub(super) fn render(text: &str, cols: usize) -> Vec<MdLine> {
    let walk = super::memo::walk(text);
    let Some(tops) = walk.tops.as_ref() else {
        return super::render(text, cols);
    };
    // The glyphs a quote bar or a bullet is drawn with follow the icon set, not the text.
    let nerd = crate::glyphs::on();
    CACHE.with(|c| {
        let mut c = c.borrow_mut();
        if c.cols != cols || c.nerd != nerd {
            *c = Cache {
                cols,
                nerd,
                pieces: HashMap::new(),
            };
        }
        // Only the pieces this version uses are kept: the rest is what the edit replaced.
        let mut old = std::mem::take(&mut c.pieces);
        let (mut out, mut blocks) = (Vec::new(), 0usize);
        for (i, &from) in tops.iter().enumerate() {
            let to = tops.get(i + 1).map_or(text.len(), |&t| t as usize);
            let src = &text[from as usize..to];
            let (key, piece) = match old.remove_entry(src) {
                Some(hit) => hit,
                None => match c.pieces.get(src) {
                    Some(p) => (src.to_string(), Rc::clone(p)),
                    None => (src.to_string(), Rc::new(lay(src, cols))),
                },
            };
            for lines in piece.iter() {
                // `layout::lines`: one blank row between top-level blocks — counted by block,
                // not by row, since a block can draw no rows and still be separated.
                if blocks > 0 {
                    out.push(MdLine {
                        spans: Vec::new(),
                        kind: LineKind::Blank,
                    });
                }
                blocks += 1;
                out.extend(lines.iter().map(|l| shifted(l, from)));
            }
            c.pieces.insert(key, piece);
        }
        out
    })
}

/// A piece's blocks, laid out the way `layout::lines` lays each block of a document.
fn lay(src: &str, cols: usize) -> Vec<Vec<MdLine>> {
    RENDERED.with(|n| n.set(n.get() + 1));
    super::parse::parse(src)
        .into_iter()
        .map(|b| super::layout::block_lines(b, cols))
        .collect()
}

/// `l` with every byte offset moved from its piece's start to the document's.
fn shifted(l: &MdLine, from: u32) -> MdLine {
    let mut l = l.clone();
    for s in &mut l.spans {
        s.src = s.src.map(|at| at + from);
    }
    l
}

/// How many pieces have been rendered afresh on this thread.
#[cfg(test)]
pub(super) fn rendered() -> u64 {
    RENDERED.with(|n| n.get())
}

#[cfg(test)]
#[path = "blockmemo_tests.rs"]
mod tests;

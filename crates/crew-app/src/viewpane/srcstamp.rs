//! Source bytes for the code and plain rungs, so a caret can stand in any text file — not only
//! in rendered markdown (see [`super::caret`]).
//!
//! Those rungs draw a line's characters in order, cut into rows, with nothing substituted: char
//! *k* of a source line is the char at byte `line_start + byte_of(k)` of the text the rung was
//! given. So each body cell is stamped with that byte here, and the rung's caller maps the
//! prepared text's bytes back to the file's ([`super::srcmap`]), as the markdown rung does.
//!
//! An empty line draws no character, and a caret needs somewhere to stand on it — in code a
//! blank line is a place you type, not a gap between blocks. It gets one [`anchor`] cell, holding
//! the byte of its own newline (or the end of the text).
use crate::chatbody::{plain, CardCell, CardLine};

/// The anchor's glyph: a space, so it draws, copies and searches as the empty line it stands on.
pub(crate) const ANCHOR_C: char = ' ';

/// A blank line's one place to stand, at byte `at`. A space is told from an anchor by being
/// bold AND italic too — a pair no code or plain row ever draws (see [`is_anchor`]).
pub(crate) fn anchor(at: u32) -> CardCell {
    let mut c = plain(ANCHOR_C, crew_theme::theme().ink, true);
    c.italic = true;
    c.src = Some(at);
    c
}

/// Whether `c` is an [`anchor`]: a stop of its own, with no stop after it. A bold-italic space in
/// markdown (`***a b***`) reads as one too, which costs it only the stop past it at a row's end.
pub(crate) fn is_anchor(c: &CardCell) -> bool {
    c.c == ANCHOR_C && c.bold && c.italic && c.src.is_some()
}

/// The stop past `cell` (at `col`, byte `src`) — none past an [`anchor`], which IS its line's end.
pub(crate) fn end_after(cell: &CardCell, col: u16, src: u32) -> Option<(u16, u32)> {
    (!is_anchor(cell)).then(|| (col, src + cell.c.len_utf8() as u32))
}

/// Where each source line of a text starts, and each of its characters' byte within the line.
pub(crate) struct Stamper {
    lines: Vec<(u32, Vec<u32>)>,
}

impl Stamper {
    pub(crate) fn new(text: &str) -> Self {
        let mut at = 0u32;
        let lines = text
            .split('\n')
            .map(|l| {
                let bytes = l.char_indices().map(|(b, _)| b as u32).collect();
                let start = at;
                at += l.len() as u32 + 1;
                (start, bytes)
            })
            .collect();
        Self { lines }
    }

    /// Stamp the `len` cells from `body_at` on in `row` with the bytes of chars `pos..pos + len`
    /// of source line `n` (1-based). A row that is a whole empty line gets an [`anchor`].
    pub(crate) fn row(&self, row: &mut CardLine, body_at: usize, n: usize, pos: usize, len: usize) {
        let Some((start, bytes)) = self.lines.get(n.wrapping_sub(1)) else {
            return;
        };
        if bytes.is_empty() {
            row.push(anchor(*start));
            return;
        }
        for (k, cell) in row.iter_mut().skip(body_at).take(len).enumerate() {
            cell.src = bytes.get(pos + k).map(|b| start + b);
        }
    }
}

#[cfg(test)]
#[path = "srcstamp_tests.rs"]
mod tests;

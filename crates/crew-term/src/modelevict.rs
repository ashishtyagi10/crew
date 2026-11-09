//! Counting the lines that fall off the top of a full scrollback.
//!
//! A picture is anchored to an absolute buffer line (`PlacedImage::line`).
//! `history_size` alone cannot number lines: it stops growing at the
//! scrollback's length, and from then on every line feed throws the oldest
//! line away while the count stays put — so a picture anchored on it froze
//! on screen while its text scrolled past. alacritty says nothing when it
//! evicts a line, so the parser runs with a little spare room above the
//! scrollback's length, and whatever spills into that room is counted and
//! trimmed off again. Child module of `model`, split for the line cap.
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::term::TermMode;

use super::TermCore;

/// The spare room, in lines. Kept within alacritty's row cache (1 000 rows,
/// `grid::storage::MAX_CACHE_SIZE`) so trimming back keeps the rows rather
/// than freeing them, and the next piece reuses them with no allocation.
const SPARE: usize = 1000;

impl TermCore {
    /// Run the parser over `bytes`, counting every line that leaves the top
    /// of the scrollback in `self.evicted`.
    pub(super) fn parse(&mut self, mut bytes: &[u8]) {
        // The alternate screen keeps no history, so nothing on it is counted
        // and it runs whole. A screen switch only ever completes on a piece's
        // FIRST byte (`advance` cuts each one just before its final byte), so
        // once past that byte the screen stays what it is.
        if self.alt() {
            let (head, rest) = bytes.split_at(bytes.len().min(1));
            self.parser.advance(&mut self.term, head);
            if self.alt() {
                self.parser.advance(&mut self.term, rest);
                return;
            }
            // Back on the primary: a cap set while it was away lands now.
            self.fit_history();
            bytes = rest;
        }
        // One byte scrolls at most a screenful (`CSI n S` is clamped to the
        // scroll region; a line feed or a wrap is one line). Bytes that
        // cannot fill the scrollback cannot evict, and run whole.
        let rows = self.term.grid().screen_lines().max(1);
        let reach = self.term.grid().history_size() + bytes.len().saturating_mul(rows);
        if reach <= self.history_cap {
            self.parser.advance(&mut self.term, bytes);
            return;
        }
        let before = self.evicted;
        while !bytes.is_empty() {
            let (piece, rest) = bytes.split_at(piece_len(bytes, rows));
            bytes = rest;
            // Only the primary's limit is raised: `grid_mut` is the active
            // screen's grid, and a piece's first byte may have switched.
            if !self.alt() {
                let room = self.history_cap + SPARE;
                self.term.grid_mut().update_history(room);
            }
            self.parser.advance(&mut self.term, piece);
            // A piece that entered the alternate screen leaves the primary
            // its room; the first piece back counts what spilled into it.
            if !self.alt() {
                let over = self
                    .term
                    .grid()
                    .history_size()
                    .saturating_sub(self.history_cap);
                self.evicted += over as u64;
                self.term.grid_mut().update_history(self.history_cap);
            }
        }
        if self.evicted != before {
            self.wipe_evicted();
        }
    }

    /// The absolute line the live screen's top row is: every line ever
    /// scrolled into the history, kept or since thrown away.
    pub(crate) fn screen_line(&self) -> u64 {
        self.evicted + self.term.grid().history_size() as u64
    }

    /// Set how many lines of scrollback the pane keeps (the `scrollback`
    /// setting, live). On the alternate screen the primary is out of reach:
    /// alacritty keeps it as a private `inactive_grid`, and `grid_mut` would
    /// hand the ALT screen a history of its own. So there only the cap moves,
    /// and the primary is cut to it on the byte that brings it back (`parse`).
    pub(crate) fn set_scrollback(&mut self, lines: usize) {
        self.history_cap = lines;
        if !self.alt() {
            self.fit_history();
        }
    }

    /// Cut the primary's history to the cap, counting what goes as evicted.
    fn fit_history(&mut self) {
        let over = self
            .term
            .grid()
            .history_size()
            .saturating_sub(self.history_cap);
        self.evicted += over as u64;
        self.term.grid_mut().update_history(self.history_cap);
        if over > 0 {
            self.wipe_evicted();
        }
    }

    fn alt(&self) -> bool {
        self.term.mode().contains(TermMode::ALT_SCREEN)
    }
}

/// How long the next piece may be so the lines it pushes fit the spare room.
/// A byte outside an escape sequence scrolls at most one line (a line feed,
/// a wrap): vte leaves its ground state only on ESC. Only a sequence's final
/// byte can scroll a screenful, and each sequence starts with an ESC — plus
/// one carried in unfinished from the piece before. So plain text runs in
/// pieces near SPARE bytes long rather than SPARE/rows: each piece costs two
/// `update_history` calls and a parser entry, and 20-byte pieces made every
/// read of a full scrollback ~14% slower. ESCs are counted 64 bytes at a time
/// (the count vectorises); a byte-at-a-time scan, or one stop per ESC, cost
/// as much as the pieces it saved.
fn piece_len(bytes: &[u8], rows: usize) -> usize {
    let fits = |len: usize, escs: usize| len + rows * (escs + 1) <= SPARE;
    let (mut len, mut escs) = (0, 0);
    for block in bytes.chunks(64) {
        let n = block.iter().filter(|&&b| b == 0x1B).count();
        if fits(len + block.len(), escs + n) {
            (len, escs) = (len + block.len(), escs + n);
            continue;
        }
        for &b in block {
            let e = escs + usize::from(b == 0x1B);
            if !fits(len + 1, e) {
                break;
            }
            (len, escs) = (len + 1, e);
        }
        break;
    }
    len.max(1)
}

#[cfg(test)]
#[path = "modelevict_tests.rs"]
mod modelevict_tests;

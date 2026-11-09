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
        // Pieces short enough that even a screenful per byte stays inside
        // the spare room, so no line is thrown away uncounted.
        for piece in bytes.chunks((SPARE / rows).max(1)) {
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

    /// Set how many lines of scrollback the pane keeps.
    pub(crate) fn set_scrollback(&mut self, lines: usize) {
        let over = self.term.grid().history_size().saturating_sub(lines);
        self.evicted += over as u64;
        self.history_cap = lines;
        self.term.grid_mut().update_history(lines);
        if over > 0 {
            self.wipe_evicted();
        }
    }

    fn alt(&self) -> bool {
        self.term.mode().contains(TermMode::ALT_SCREEN)
    }
}

#[cfg(test)]
#[path = "modelevict_tests.rs"]
mod modelevict_tests;

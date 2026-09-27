//! Which viewport rows soft-wrap onto the next — the one fact about a row
//! its text cannot tell you.
//!
//! A path or URL longer than the pane is written across two rows, and a
//! scanner reading row by row sees two unrelated halves: Cmd+E labelled
//! `fold.rs:25` and opened a file that does not exist, because
//! `crates/crew-app/src/md/` was the row above. Alacritty marks the last
//! cell of a row the program ran past with `WRAPLINE`; this reads it.
use super::*;

impl TermCore {
    /// One flag per viewport row: whether that row continues on the next.
    pub(crate) fn wrapped_rows(&self) -> Vec<bool> {
        let content = self.term.renderable_content();
        let off = content.display_offset as i32;
        let rows = self.term.screen_lines();
        let mut out = vec![false; rows];
        for ind in content.display_iter {
            let line = ind.point.line.0 + off;
            if line >= 0 && ind.flags.contains(Flags::WRAPLINE) {
                if let Some(slot) = out.get_mut(line as usize) {
                    *slot = true;
                }
            }
        }
        out
    }
}

#[cfg(test)]
#[path = "modelwrap_tests.rs"]
mod modelwrap_tests;

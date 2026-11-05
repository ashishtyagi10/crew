//! A program learns how big the pane is in pixels.
//!
//! kitty `icat`, chafa, timg, yazi and notcurses size a picture to the text
//! area, and they ask three ways: the pty's own size (`TIOCGWINSZ`, whose
//! pixel fields crew left 0), `CSI 14 t` (the text area in pixels, which
//! alacritty raises as an event crew dropped) and `CSI 16 t` (one cell in
//! pixels, which the parser does not dispatch at all). With no answer they
//! guess a cell size or fall back to block characters, so the pictures crew
//! draws came out the wrong size. All three now answer from one record, the
//! grid and the cell size the app publishes on every relayout.
use alacritty_terminal::event::WindowSize;
use alacritty_terminal::grid::Dimensions;
use portable_pty::PtySize;

use super::TermCore;

impl TermCore {
    /// Record the grid and cell size where `CSI 14 t` is answered from.
    pub(crate) fn publish_size(&self) {
        let (w, h) = self.cell_px;
        let px = |n: u32| n.min(u16::MAX.into()) as u16;
        *self.events.size.lock().unwrap() = Some(WindowSize {
            num_lines: self.term.screen_lines() as u16,
            num_cols: self.term.columns() as u16,
            cell_width: px(w),
            cell_height: px(h),
        });
    }

    /// The pty's size with its pixel fields: the text area, cells times
    /// the cell size, as xterm and kitty report it.
    pub(crate) fn pty_size(&self) -> PtySize {
        let s = self
            .events
            .size
            .lock()
            .unwrap()
            .expect("set in TermCore::new");
        PtySize {
            rows: s.num_lines,
            cols: s.num_cols,
            pixel_width: s.num_cols.saturating_mul(s.cell_width),
            pixel_height: s.num_lines.saturating_mul(s.cell_height),
        }
    }
}

/// The scanner that answers `CSI 16 t` (and XTVERSION) from the raw bytes.
#[path = "modelquery.rs"]
mod modelquery;
pub(super) use modelquery::Queries;

#[cfg(test)]
#[path = "modelsize_tests.rs"]
mod tests;

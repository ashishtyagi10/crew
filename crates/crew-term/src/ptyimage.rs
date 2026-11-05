//! The picture half of a pane's output (what to show and what to drop), and
//! the scrollback search. Child module of `pty`, split for the 200-line cap.
use super::*;

impl PtyTerm {
    /// Take the pictures this pane's program asked to show since the last
    /// call. Raw payloads: decoding one on this thread would freeze the grid.
    pub fn take_images(&mut self) -> Vec<crate::model::PlacedImage> {
        self.core.take_images()
    }

    /// Take the pictures the program dropped since the last call (`clear`,
    /// the alternate screen, a kitty delete). Apply before `take_images`'.
    pub fn take_image_wipes(&mut self) -> Vec<crate::ImageWipe> {
        self.core.take_image_wipes()
    }

    /// The newest logical line at or above depth `from` (lines above the
    /// screen's bottom row) that `hit` accepts, as the depth of its top row.
    /// Reads the grid in one pass without moving the view (see `modelscroll`).
    pub fn find_back(&self, from: usize, hit: &mut dyn FnMut(&str) -> bool) -> Option<usize> {
        self.core.find_back(from, hit)
    }
}

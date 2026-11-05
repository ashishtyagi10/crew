//! The picture half of a pane's output: what to show and what to drop.
//! Child module of `pty`, split for the 200-line cap.
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
}

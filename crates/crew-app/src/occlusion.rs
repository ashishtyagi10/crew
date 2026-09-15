//! Whether a frame is worth asking for.
//!
//! The compositor hides a window for ordinary reasons — another window over
//! it, a minimise, a display that has gone to sleep — and none of them stop
//! the panes underneath. Output keeps arriving, each arrival asks for a
//! redraw, and every one of those frames is built, shaped and handed to a
//! surface that refuses it.
//!
//! That refusal used to be expensive (see `crew-render`'s `frame::drain`);
//! now it is merely pointless, which is still a reason not to do it for the
//! eight hours a laptop spends asleep on a desk.
use crate::app::CrewApp;

impl CrewApp {
    /// Ask for a frame, unless nobody can see one.
    pub(crate) fn redraw(&self) {
        if !self.wants_frame() {
            return;
        }
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    /// The compositor says whether this window is visible. Coming back asks
    /// for one frame directly — the window has to repaint, and `redraw` was
    /// answering nothing a moment ago.
    pub(crate) fn set_occluded(&mut self, hidden: bool) {
        self.occluded = hidden;
        if !hidden {
            if let Some(w) = &self.window {
                w.request_redraw();
            }
        }
    }

    /// Is a frame worth building at all? The one question `redraw` asks, kept
    /// separate so it can be asserted without a window to ask.
    pub(crate) fn wants_frame(&self) -> bool {
        !self.occluded
    }
}

#[cfg(test)]
#[path = "occlusion_tests.rs"]
mod tests;

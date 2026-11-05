//! The window's resize and scale-change handlers, moved out of `events` so
//! the dispatch there stays a list of arms rather than their bodies.
use std::time::Instant;

use winit::dpi::PhysicalSize;

use crate::app::CrewApp;

impl CrewApp {
    /// The surface follows the new size, and the logical size is remembered
    /// to persist (debounced in `poll_panes`) — except while maximized, so the
    /// restore size stays the un-maximized one.
    pub(crate) fn on_resized(&mut self, size: PhysicalSize<u32>) {
        if let Some(renderer) = &mut self.renderer {
            renderer.resize(size.width, size.height);
        }
        if let Some(w) = &self.window {
            if !w.is_maximized() {
                let scale = w.scale_factor() as f32;
                self.config.win_w = Some(size.width as f32 / scale);
                self.config.win_h = Some(size.height as f32 / scale);
                self.resize_at = Some(Instant::now());
            }
        }
        self.redraw();
    }

    /// A move to a display of another density re-sizes the font in pixels.
    pub(crate) fn on_scale_changed(&mut self, scale_factor: f64) {
        if let Some(renderer) = &mut self.renderer {
            renderer.set_font_size(self.config.font_size * scale_factor as f32);
            renderer.set_leading(self.config.leading().ratio());
        }
        self.redraw();
    }
}

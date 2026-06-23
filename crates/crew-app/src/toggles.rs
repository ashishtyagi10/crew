//! Palette-discoverable toggles that mirror the Cmd-chord shortcuts: `/broadcast`
//! (Cmd+S), `/zoom` (Cmd+Z), `/sidebar` (Cmd+G). The fuzzy command palette
//! surfaces them by name when you can't recall the chord. The chords call the
//! same methods, so behaviour stays in lockstep.
use crate::app::CrewApp;
use crate::chords::broadcast_label;

impl CrewApp {
    /// Toggle broadcast — mirror typed input to every terminal pane.
    pub(crate) fn toggle_broadcast(&mut self) {
        self.broadcast = !self.broadcast;
        self.input.broadcast = self.broadcast;
        self.set_status(broadcast_label(self.broadcast));
        self.redraw();
    }

    /// Toggle zoom — the focused pane fills the content area.
    pub(crate) fn toggle_zoom(&mut self) {
        self.zoomed = !self.zoomed;
        self.set_status(if self.zoomed { "zoomed" } else { "unzoomed" });
        self.redraw();
    }
}

#[cfg(test)]
mod tests {
    use crate::app::CrewApp;

    #[test]
    fn toggle_broadcast_flips_and_mirrors_input() {
        let mut app = CrewApp::default();
        assert!(!app.broadcast && !app.input.broadcast);
        app.toggle_broadcast();
        assert!(app.broadcast && app.input.broadcast);
        app.toggle_broadcast();
        assert!(!app.broadcast && !app.input.broadcast);
    }

    #[test]
    fn toggle_zoom_flips() {
        let mut app = CrewApp::default();
        app.toggle_zoom();
        assert!(app.zoomed);
        app.toggle_zoom();
        assert!(!app.zoomed);
    }
}

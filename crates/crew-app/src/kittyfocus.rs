//! Releases owed to a pane focus has left (see [`crate::kittyheld`]).
use crate::app::CrewApp;
use crate::pane::PaneContent;

impl CrewApp {
    /// Write the releases a kitty-flagged pane is still owed for keys pressed
    /// there. `blur` (the window lost focus) owes every pane of this window;
    /// otherwise only panes that are no longer the focused one. Cheap when
    /// nothing is held, so every window event can call it.
    pub(crate) fn release_stale_keys(&mut self, blur: bool) {
        let now = self.panes.get(self.focused).map(|p| p.born_ms);
        let mine: Vec<u64> = self.panes.iter().map(|p| p.born_ms).collect();
        let owed = crate::kittyheld::take_stale(|p| mine.contains(&p), |p| !blur && Some(p) == now);
        for (pane, bytes) in owed {
            let Some(p) = self.panes.iter_mut().find(|p| p.born_ms == pane) else {
                continue;
            };
            if let PaneContent::Terminal(t) = &mut p.content {
                use std::io::Write;
                let _ = t.input.write_all(&bytes).and_then(|_| t.input.flush());
            }
        }
    }
}

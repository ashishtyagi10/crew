//! Routing submitted input to terminal panes: the focused one, or every
//! terminal pane when broadcast (synchronized input) is on.
use std::io::Write;

use crate::app::CrewApp;
use crate::pane::PaneContent;

impl CrewApp {
    /// Write `bytes` to the focused terminal — or, when broadcast is on, to every
    /// terminal pane. Each write snaps to the bottom. Returns how many terminals
    /// received it (0 means nothing did, e.g. no shell is open/focused).
    pub(crate) fn write_to_terminals(&mut self, bytes: &[u8]) -> usize {
        let all = self.broadcast;
        let focused = self.focused;
        let mut count = 0;
        for (i, pane) in self.panes.iter_mut().enumerate() {
            if !all && i != focused {
                continue;
            }
            if let PaneContent::Terminal(t) = &mut pane.content {
                t.pty.scroll_to_bottom();
                if let Err(e) = t.input.write_all(bytes).and_then(|_| t.input.flush()) {
                    eprintln!("terminal write error: {e}");
                } else {
                    count += 1;
                }
            }
        }
        count
    }
}

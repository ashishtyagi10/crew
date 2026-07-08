//! Routing submitted input to terminal panes: the focused one, or every
//! terminal pane when broadcast (synchronized input) is on.
use std::io::Write;

use crate::app::CrewApp;
use crate::pane::PaneContent;

impl CrewApp {
    /// Write `bytes` to terminal panes: the focused one, or — when `all` — every
    /// terminal pane. Each write snaps to the bottom. Returns how many terminals
    /// received it (0 means nothing did, e.g. no shell is open/focused).
    pub(crate) fn write_terminal_targets(&mut self, bytes: &[u8], all: bool) -> usize {
        let focused = self.focused;
        let mut count = 0;
        for (i, pane) in self.panes.iter_mut().enumerate() {
            if !all && i != focused {
                continue;
            }
            if let PaneContent::Terminal(t) = &mut pane.content {
                t.pty.scroll_to_bottom();
                // Typing invalidates any mouse selection — drop the stale
                // highlight so it doesn't linger painted over fresh output.
                t.pty.sel_clear();
                if let Err(e) = t.input.write_all(bytes).and_then(|_| t.input.flush()) {
                    eprintln!("terminal write error: {e}");
                } else {
                    count += 1;
                }
            }
        }
        count
    }

    /// Keystrokes typed while a terminal pane is focused: honors Cmd+S broadcast
    /// (synchronized typing). The input bar does NOT come through here — its
    /// routing never consults the mode.
    pub(crate) fn write_to_terminals(&mut self, bytes: &[u8]) -> usize {
        let all = self.broadcast;
        self.write_terminal_targets(bytes, all)
    }
}

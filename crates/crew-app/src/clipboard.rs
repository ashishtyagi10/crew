//! Clipboard paste into the focused terminal pane.
use std::io::Write;

use crate::app::CrewApp;
use crate::pane::PaneContent;

impl CrewApp {
    /// Paste the system clipboard text into the focused terminal pane, using
    /// bracketed paste when the running program enabled it.
    pub(crate) fn paste(&mut self) {
        let Ok(mut cb) = arboard::Clipboard::new() else {
            return;
        };
        let Ok(text) = cb.get_text() else {
            return;
        };
        if text.is_empty() {
            return;
        }
        let focused = self.focused;
        if let Some(pane) = self.panes.get_mut(focused) {
            if let PaneContent::Terminal(t) = &mut pane.content {
                let bytes = crate::session::wrap_paste(&text, t.pty.bracketed_paste());
                t.pty.scroll_to_bottom();
                if let Err(e) = t.input.write_all(&bytes).and_then(|_| t.input.flush()) {
                    eprintln!("paste write error: {e}");
                }
            }
        }
        self.redraw();
    }
}

//! Clipboard paste into the focused surface (input bar, chat, or terminal).
use std::io::Write;

use crate::app::CrewApp;
use crate::pane::PaneContent;

/// Flatten clipboard text to a single line for single-line inputs.
fn one_line(s: &str) -> String {
    s.replace(['\n', '\r'], " ")
}

impl CrewApp {
    /// Paste the system clipboard into the focused surface: the command input
    /// bar, a chat pane's input (single-line), or the focused terminal (using
    /// bracketed paste when the running program enabled it).
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
        if self.input.focused {
            self.input.text.push_str(&one_line(&text));
            self.redraw();
            return;
        }
        if let Some(pane) = self.panes.get_mut(self.focused) {
            match &mut pane.content {
                PaneContent::Terminal(t) => {
                    let bytes = crate::session::wrap_paste(&text, t.pty.bracketed_paste());
                    t.pty.scroll_to_bottom();
                    if let Err(e) = t.input.write_all(&bytes).and_then(|_| t.input.flush()) {
                        eprintln!("paste write error: {e}");
                    }
                }
                PaneContent::Chat(c) => c.input.push_str(&one_line(&text)),
                PaneContent::Settings(_) => {}
            }
        }
        self.redraw();
    }

    /// Take a pending OSC 52 clipboard-store request from any terminal pane.
    pub(crate) fn take_pane_clipboard(&self) -> Option<String> {
        self.panes.iter().find_map(|p| match &p.content {
            PaneContent::Terminal(t) => t.pty.take_clipboard(),
            _ => None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::one_line;

    #[test]
    fn one_line_flattens_newlines() {
        assert_eq!(one_line("a\nb\r\nc"), "a b  c");
        assert_eq!(one_line("plain"), "plain");
    }
}

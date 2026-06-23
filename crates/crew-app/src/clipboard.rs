//! Clipboard copy/paste for the focused surface (input bar, chat, or terminal).
use std::io::Write;

use crate::app::CrewApp;
use crate::pane::PaneContent;
use crew_term::{RenderCell, TermModel};

/// Reconstruct the visible terminal screen as text: each row trimmed of trailing
/// spaces, with trailing blank rows dropped.
fn screen_text(cells: &[RenderCell], cols: u16, rows: u16) -> String {
    let mut lines: Vec<String> = Vec::new();
    for r in 0..rows {
        let mut line = vec![' '; cols as usize];
        for cell in cells.iter().filter(|c| c.row == r) {
            if (cell.col as usize) < line.len() {
                line[cell.col as usize] = cell.c;
            }
        }
        lines.push(line.into_iter().collect::<String>().trim_end().to_string());
    }
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    lines.join("\n")
}

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
                PaneContent::Settings(_) | PaneContent::Far(_) => {}
            }
        }
        self.redraw();
    }

    /// Copy the focused terminal's visible screen to the system clipboard,
    /// flashing a status message with the line count.
    pub(crate) fn copy_screen(&mut self) {
        let Some(pane) = self.panes.get(self.focused) else {
            return;
        };
        if let PaneContent::Terminal(t) = &pane.content {
            let text = screen_text(&t.pty.cells(false), pane.grid.cols, pane.grid.rows);
            if !text.is_empty() {
                if let Ok(mut cb) = arboard::Clipboard::new() {
                    let lines = text.lines().count();
                    let _ = cb.set_text(text);
                    self.set_status(format!("copied {lines} lines"));
                }
            }
        }
    }

    /// Copy the focused terminal's full scrollback to the clipboard (`/copy`).
    /// Unlike Cmd+C (visible screen only), this walks the entire history.
    pub(crate) fn copy_scrollback(&mut self) {
        let focused = self.focused;
        let Some(pane) = self.panes.get_mut(focused) else {
            self.set_status("copy: focus a terminal pane");
            return;
        };
        let (cols, rows) = (pane.grid.cols, pane.grid.rows);
        let PaneContent::Terminal(t) = &mut pane.content else {
            self.set_status("copy: focus a terminal pane");
            return;
        };
        let text = crate::dump::capture_scrollback(&mut t.pty, cols, rows);
        if text.trim().is_empty() {
            self.set_status("nothing to copy");
            return;
        }
        if let Ok(mut cb) = arboard::Clipboard::new() {
            let lines = text.lines().count();
            let _ = cb.set_text(text);
            self.set_status(format!("copied {lines} lines (scrollback)"));
        }
    }

    /// Copy Crew's working directory to the system clipboard (`/pwd`), since the
    /// legend isn't selectable. Flashes the `~`-abbreviated path.
    pub(crate) fn copy_cwd(&mut self) {
        if self.cwd.as_os_str().is_empty() {
            self.set_status("no working directory");
            return;
        }
        let full = self.cwd.to_string_lossy().into_owned();
        if let Ok(mut cb) = arboard::Clipboard::new() {
            let _ = cb.set_text(full);
            self.set_status(format!("copied {}", crate::cwd::display(&self.cwd)));
        }
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
    use super::{one_line, screen_text};

    #[test]
    fn one_line_flattens_newlines() {
        assert_eq!(one_line("a\nb\r\nc"), "a b  c");
        assert_eq!(one_line("plain"), "plain");
    }

    #[test]
    fn screen_text_trims_and_drops_blank_tail() {
        use crew_term::RenderCell;
        let c = |col, row, ch| RenderCell {
            col,
            row,
            c: ch,
            fg: (0, 0, 0),
            bg: (0, 0, 0),
            bold: false,
            italic: false,
        };
        // "hi" on row 0, "x" on row 1, row 2 blank → trailing blank dropped.
        let cells = [c(0, 0, 'h'), c(1, 0, 'i'), c(0, 1, 'x')];
        assert_eq!(screen_text(&cells, 5, 3), "hi\nx");
    }
}

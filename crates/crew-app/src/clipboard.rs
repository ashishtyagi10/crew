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

/// Normalize clipboard line endings to `\n` for the multiline chat composer.
fn multiline(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\r', "\n")
}

/// Route a paste into a chat pane: while its masked key prompt
/// ([`crate::keyentry::KeyEntry`]) is open, EVERY pasted character goes to
/// the prompt's buffer, never the composer — the prompt is modal, and a key
/// pasted while it's up must never land in the visible transcript-bound
/// input. Only once no prompt is open does a paste fall through to the
/// ordinary multiline composer.
fn paste_into_chat(c: &mut crate::chat::ChatPane, text: &str) {
    match c.keyentry.as_mut() {
        Some(entry) => entry.paste(text),
        None => c.input.push_str(&multiline(text)),
    }
}

/// A pane's drawn cells as text, trailing blanks trimmed — the same reading
/// `screen_text` gives a terminal grid, for the panes crew draws itself.
fn rendered_text(cells: &[crew_render::CellView], cols: u16, rows: u16) -> String {
    crate::gridrows::grid_lines(cells, cols, rows)
        .into_iter()
        .map(|line| line.iter().collect::<String>().trim_end().to_string())
        .collect::<Vec<_>>()
        .join("\n")
        .trim_end()
        .to_string()
}

impl CrewApp {
    /// Paste the system clipboard into the focused surface: the command input
    /// bar, a chat pane's input (multiline), or the focused terminal (using
    /// bracketed paste when the running program enabled it). When the clipboard
    /// holds an image (and no text), it's saved to a temp PNG and the file path
    /// is pasted instead — so agent CLIs can read the image by path.
    pub(crate) fn paste(&mut self) {
        let Ok(mut cb) = arboard::Clipboard::new() else {
            return;
        };
        let text = match cb.get_text() {
            Ok(t) if !t.is_empty() => t,
            _ => match self.paste_image(&mut cb) {
                Some(path) => path,
                None => return,
            },
        };
        // A held paste is answered by the next Cmd+V — the same key, which is
        // the one people press when they mean "yes, that one".
        if let Some(held) = self.held_paste.take(std::time::Instant::now()) {
            self.insert_paste(&held);
            return;
        }
        let bracketed = match self.panes.get(self.focused).map(|p| &p.content) {
            Some(PaneContent::Terminal(t)) if !self.input.focused => Some(t.pty.bracketed_paste()),
            _ => None,
        };
        // Only a terminal runs what it is handed; the bar, a chat composer and
        // a todo list all just hold the text.
        if let Some(false) = bracketed {
            if crate::pastesafe::needs_confirm(&text, false) {
                let n = crate::pastesafe::line_count(&text);
                self.held_paste.hold(&text, std::time::Instant::now());
                self.set_status(format!(
                    "{n} lines would run here \u{2014} \u{2318}V again to paste"
                ));
                self.redraw();
                return;
            }
        }
        self.insert_paste(&text);
    }

    /// Insert pasted `text` into the focused surface and redraw.
    fn insert_paste(&mut self, text: &str) {
        if self.input.focused {
            self.input.text.push_str(&one_line(text));
            self.redraw();
            return;
        }
        if let Some(pane) = self.panes.get_mut(self.focused) {
            match &mut pane.content {
                PaneContent::Terminal(t) => {
                    let bytes = crate::session::wrap_paste(text, t.pty.bracketed_paste());
                    t.pty.scroll_to_bottom();
                    if let Err(e) = t.input.write_all(&bytes).and_then(|_| t.input.flush()) {
                        eprintln!("paste write error: {e}");
                    }
                }
                PaneContent::Chat(c) => paste_into_chat(c, text),
                PaneContent::Todo(t) => t.paste(text),
                PaneContent::Settings(_)
                | PaneContent::Far(_)
                | PaneContent::Swarm(_)
                | PaneContent::Usage(_)
                | PaneContent::View(_) => {}
            }
        }
        self.redraw();
    }

    /// Save a clipboard image to a temp PNG, returning its path as a string to
    /// paste. `None` when there's no image or it can't be written.
    fn paste_image(&mut self, cb: &mut arboard::Clipboard) -> Option<String> {
        let img = cb.get_image().ok()?;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let path = std::env::temp_dir().join(format!("crew-paste-{stamp}.png"));
        let buf = image::RgbaImage::from_raw(
            img.width as u32,
            img.height as u32,
            img.bytes.into_owned(),
        )?;
        buf.save(&path).ok()?;
        self.set_status(format!("pasted image → {}", path.display()));
        Some(path.to_string_lossy().into_owned())
    }

    /// Copy the focused terminal's visible screen to the system clipboard,
    /// flashing a status message with the line count.
    pub(crate) fn copy_screen(&mut self) {
        // An active mouse selection wins over the whole-screen copy.
        if let Some(text) = self.pane_selection_text(self.focused) {
            self.copy_text(text);
            return;
        }
        let Some(pane) = self.panes.get(self.focused) else {
            return;
        };
        let (cols, rows) = (pane.grid.cols, pane.grid.rows);
        // Whatever the pane is, copy what it SHOWS. Only terminals used to
        // answer this chord: in a viewer, a diff, a transcript or a todo list
        // — every pane kind crew has that is worth reading — Cmd+C did
        // nothing at all and said nothing about it.
        let text = match &pane.content {
            PaneContent::Terminal(t) => screen_text(&t.pty.cells(false), cols, rows),
            _ => rendered_text(&pane.cells(true), cols, rows),
        };
        if text.trim().is_empty() {
            self.set_status("nothing on screen to copy");
            return;
        }
        match arboard::Clipboard::new() {
            Ok(mut cb) => {
                let lines = text.lines().count();
                let _ = cb.set_text(text);
                self.set_status(format!("copied {lines} lines"));
            }
            Err(_) => self.set_status("clipboard unavailable"),
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

    /// `/copy out` — the last command's output alone, rather than the whole
    /// scrollback. What you actually want when you are about to paste a
    /// failure into an issue: the run that failed, without the four before it.
    pub(crate) fn copy_last_output(&mut self) {
        let (name, text) = match self.last_output() {
            Ok(v) => v,
            Err(why) => {
                self.set_status(format!("copy: {why}"));
                return;
            }
        };
        let lines = text.lines().count();
        match arboard::Clipboard::new() {
            Ok(mut cb) => {
                let _ = cb.set_text(text);
                self.set_status(format!("copied {lines} lines ({name})"));
            }
            Err(_) => self.set_status("clipboard unavailable"),
        }
    }

    /// Copy Crew's working directory to the system clipboard (`/pwd`).
    pub(crate) fn copy_cwd(&mut self) {
        let dir = self.cwd.display().to_string();
        match arboard::Clipboard::new() {
            Ok(mut cb) => {
                let _ = cb.set_text(dir.clone());
                self.set_status(format!("copied cwd: {dir}"));
            }
            Err(_) => self.set_status("clipboard unavailable"),
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
mod screen_tests {
    use super::rendered_text;
    use crew_render::CellView;

    fn cell(col: u16, row: u16, c: char) -> CellView {
        CellView {
            col,
            row,
            c,
            ..Default::default()
        }
    }

    /// Every pane crew draws itself can be copied now, and what comes out is
    /// what is on the screen — rows in order, gaps as spaces, no trailing
    /// padding.
    #[test]
    fn a_drawn_pane_reads_back_as_its_rows() {
        let cells = vec![cell(0, 0, 'h'), cell(1, 0, 'i'), cell(2, 1, 'x')];
        assert_eq!(rendered_text(&cells, 6, 3), "hi\n  x");
    }

    /// A blank pane copies nothing rather than a block of spaces.
    #[test]
    fn a_blank_pane_reads_back_empty() {
        assert_eq!(rendered_text(&[], 10, 4), "");
        assert_eq!(rendered_text(&[cell(3, 1, ' ')], 10, 4), "");
    }

    /// Cells outside the grid are not part of the screen.
    #[test]
    fn cells_past_the_grid_are_left_out() {
        let cells = vec![cell(0, 0, 'a'), cell(99, 0, 'b'), cell(0, 99, 'c')];
        assert_eq!(rendered_text(&cells, 4, 2), "a");
    }
}

#[cfg(test)]
mod tests {
    use super::{one_line, paste_into_chat, screen_text};

    #[test]
    fn one_line_flattens_newlines() {
        assert_eq!(one_line("a\nb\r\nc"), "a b  c");
        assert_eq!(one_line("plain"), "plain");
    }

    #[test]
    fn multiline_keeps_newlines_and_normalizes_crlf() {
        use super::multiline;
        assert_eq!(multiline("a\r\nb\rc\nd"), "a\nb\nc\nd");
        assert_eq!(multiline("plain"), "plain");
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
            ..Default::default()
        };
        // "hi" on row 0, "x" on row 1, row 2 blank → trailing blank dropped.
        let cells = [c(0, 0, 'h'), c(1, 0, 'i'), c(0, 1, 'x')];
        assert_eq!(screen_text(&cells, 5, 3), "hi\nx");
    }

    // Regression for the CRITICAL finding: with the masked key prompt open,
    // Cmd+V / right-click paste must never land in the visible composer —
    // `insert_paste` (the actual paste entry point) isn't reachable from a
    // unit test without constructing a whole `CrewApp` + windowed pane, so
    // this drives `paste_into_chat`, the routing helper both real paste
    // sources (`chords.rs`'s Cmd+V and `events.rs`'s right-click) funnel
    // through via `insert_paste`.
    #[test]
    fn paste_goes_to_an_open_key_prompt_not_the_composer() {
        let mut p = crate::chat::tests::pane();
        p.keyentry = Some(crate::keyentry::KeyEntry::new("ANTHROPIC_API_KEY".into()));
        let secret = "sk-pasted-secret";
        paste_into_chat(&mut p, &format!("{secret}\n"));

        assert!(
            p.input.is_empty(),
            "the composer must stay untouched while the prompt is open"
        );
        let masked = p
            .keyentry
            .as_ref()
            .unwrap()
            .card(60)
            .iter()
            .filter(|cell| cell.c == '•')
            .count();
        assert_eq!(
            masked,
            secret.chars().count(),
            "the pasted text (minus the trailing newline) reached the prompt's buffer"
        );
    }

    #[test]
    fn paste_reaches_the_composer_when_no_prompt_is_open() {
        let mut p = crate::chat::tests::pane();
        paste_into_chat(&mut p, "hello\nworld");
        assert_eq!(
            p.input, "hello\nworld",
            "no prompt open: ordinary paste path"
        );
    }
}

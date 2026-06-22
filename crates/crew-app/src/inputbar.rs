//! Docked bottom command bar: a single-line text input. The surrounding pane
//! draws the rounded border (so it bottom-aligns with the sidebar/panes); this
//! only renders the `> text` content inside it.
use crew_render::CellView;
use winit::keyboard::{Key, NamedKey};

const BG: (u8, u8, u8) = (0, 0, 0);
const ACCENT: (u8, u8, u8) = (0, 255, 160);
const DIM: (u8, u8, u8) = (120, 130, 140);
const TEXT_FG: (u8, u8, u8) = (220, 220, 220);

#[derive(Default)]
pub struct InputBar {
    pub text: String,
    pub focused: bool,
    /// Submitted lines, oldest first — the source for history autosuggestions.
    pub history: Vec<String>,
    /// Highlighted row in the command palette (when it's open).
    pub menu_sel: usize,
    /// Position while browsing history with Up/Down (`None` = editing fresh text).
    pub hist_pos: Option<usize>,
}

impl InputBar {
    /// Render `> text` vertically centered inside the input pane. The prompt is
    /// accent-green when focused, dim otherwise.
    pub fn cells(&self, cols: u16, rows: u16) -> Vec<CellView> {
        if cols < 4 || rows == 0 {
            return Vec::new();
        }
        let row = rows / 2;
        let start = 2u16;
        let prompt_fg = if self.focused { ACCENT } else { DIM };
        // Drawable columns after the gutter; the first 2 hold the "> " prompt.
        let max = cols.saturating_sub(start + 1) as usize;
        let text_area = max.saturating_sub(2);
        // Typed text (bright), then either the ghost suggestion (dim) or the
        // block cursor when there's nothing to suggest.
        let mut body: Vec<(char, (u8, u8, u8))> = self.text.chars().map(|c| (c, TEXT_FG)).collect();
        // Ghost text: when the command palette is open, mirror the highlighted
        // command; otherwise fall back to history/slash autosuggestion.
        let ghost = if !self.focused {
            None
        } else {
            let m = crate::suggest::matches(&self.text);
            if m.is_empty() {
                crate::suggest::suggest(&self.text, &self.history)
            } else {
                let name = m[self.menu_sel.min(m.len() - 1)].name;
                Some(name[self.text.len()..].to_string())
            }
        };
        match &ghost {
            Some(g) => body.extend(g.chars().map(|c| (c, DIM))),
            None if self.focused => body.push(('█', ACCENT)),
            None => {}
        }
        // Follow the cursor: when the body overflows the field, show its tail.
        let skip = body.len().saturating_sub(text_area);
        let mut out = Vec::new();
        for (i, ch) in "> ".chars().enumerate() {
            out.push(CellView {
                col: start + i as u16,
                row,
                c: ch,
                fg: prompt_fg,
                bg: BG,
                bold: false,
                italic: false,
            });
        }
        for (i, &(ch, fg)) in body[skip..].iter().enumerate() {
            out.push(CellView {
                col: start + 2 + i as u16,
                row,
                c: ch,
                fg,
                bg: BG,
                bold: false,
                italic: false,
            });
        }
        out
    }

    /// Handle a winit key event: translate and delegate to `input_reduce`.
    ///
    /// Returns `Some(submitted_line)` when Enter is pressed (the text before clearing),
    /// or `None` for all other keys.
    pub fn on_key(&mut self, key: &winit::event::KeyEvent) -> Option<String> {
        if !key.state.is_pressed() {
            return None;
        }
        let menu = crate::suggest::matches(&self.text);
        let menu_open = self.focused && !menu.is_empty();

        // Command-palette navigation.
        if menu_open {
            match &key.logical_key {
                Key::Named(NamedKey::ArrowDown) => {
                    self.menu_sel = (self.menu_sel + 1) % menu.len();
                    return None;
                }
                Key::Named(NamedKey::ArrowUp) => {
                    self.menu_sel = (self.menu_sel + menu.len() - 1) % menu.len();
                    return None;
                }
                _ => {}
            }
        }

        // Up/Down recall submitted history (when the palette isn't open).
        if !menu_open {
            match &key.logical_key {
                Key::Named(NamedKey::ArrowUp) => {
                    self.history_prev();
                    return None;
                }
                Key::Named(NamedKey::ArrowDown) => {
                    self.history_next();
                    return None;
                }
                _ => {}
            }
        }

        // Tab / Right accept: the highlighted command, else the ghost suffix.
        if matches!(
            &key.logical_key,
            Key::Named(NamedKey::Tab) | Key::Named(NamedKey::ArrowRight)
        ) {
            if menu_open {
                self.text = menu[self.menu_sel.min(menu.len() - 1)].name.to_string();
            } else if let Some(g) = crate::suggest::suggest(&self.text, &self.history) {
                self.text.push_str(&g);
            }
            self.menu_sel = 0;
            return None;
        }

        // Enter on an open palette runs the highlighted command.
        if menu_open && matches!(&key.logical_key, Key::Named(NamedKey::Enter)) {
            let name = menu[self.menu_sel.min(menu.len() - 1)].name.to_string();
            self.history.push(name.clone());
            self.text.clear();
            self.menu_sel = 0;
            return Some(name);
        }

        let (ch, enter, backspace) = match &key.logical_key {
            Key::Named(NamedKey::Enter) => (None, true, false),
            Key::Named(NamedKey::Backspace) => (None, false, true),
            Key::Named(NamedKey::Space) => (Some(' '), false, false),
            Key::Character(s) => (s.chars().next(), false, false),
            _ => (None, false, false),
        };
        let result = crate::chatlayout::input_reduce(&mut self.text, ch, enter, backspace);
        self.menu_sel = 0; // editing changes the match set; re-highlight the top
        self.hist_pos = None; // editing leaves history-browse mode
        if let Some(line) = &result {
            if !line.is_empty() {
                self.history.push(line.clone());
            }
        }
        result
    }

    /// Step to an older history entry (Up), loading it into the input.
    fn history_prev(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let i = match self.hist_pos {
            None => self.history.len() - 1,
            Some(i) => i.saturating_sub(1),
        };
        self.hist_pos = Some(i);
        self.text = self.history[i].clone();
    }

    /// Step to a newer history entry (Down); past the newest clears the input.
    fn history_next(&mut self) {
        match self.hist_pos {
            Some(i) if i + 1 < self.history.len() => {
                self.hist_pos = Some(i + 1);
                self.text = self.history[i + 1].clone();
            }
            Some(_) => {
                self.hist_pos = None;
                self.text.clear();
            }
            None => {}
        }
    }
}

#[cfg(test)]
#[path = "inputbar_tests.rs"]
mod tests;

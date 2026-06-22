//! Docked bottom command bar: a single-line text input. The surrounding pane
//! draws the rounded border (so it bottom-aligns with the sidebar/panes); this
//! only renders the `> text` content inside it.
use crew_render::CellView;
use winit::keyboard::{Key, NamedKey};

const BG: (u8, u8, u8) = (8, 8, 16);
const ACCENT: (u8, u8, u8) = (0, 255, 160);
const DIM: (u8, u8, u8) = (120, 130, 140);
const TEXT_FG: (u8, u8, u8) = (220, 220, 220);

#[derive(Default)]
pub struct InputBar {
    pub text: String,
    pub focused: bool,
    /// Submitted lines, oldest first — the source for history autosuggestions.
    pub history: Vec<String>,
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
        let ghost = if self.focused {
            crate::suggest::suggest(&self.text, &self.history)
        } else {
            None
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
        // Tab / Right accept the type-ahead suggestion.
        if matches!(
            &key.logical_key,
            Key::Named(NamedKey::Tab) | Key::Named(NamedKey::ArrowRight)
        ) {
            if let Some(g) = crate::suggest::suggest(&self.text, &self.history) {
                self.text.push_str(&g);
            }
            return None;
        }
        let (ch, enter, backspace) = match &key.logical_key {
            Key::Named(NamedKey::Enter) => (None, true, false),
            Key::Named(NamedKey::Backspace) => (None, false, true),
            Key::Named(NamedKey::Space) => (Some(' '), false, false),
            Key::Character(s) => (s.chars().next(), false, false),
            _ => (None, false, false),
        };
        let result = crate::chatlayout::input_reduce(&mut self.text, ch, enter, backspace);
        if let Some(line) = &result {
            if !line.is_empty() {
                self.history.push(line.clone());
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cells_focused_shows_accent_prompt_and_text() {
        let bar = InputBar {
            text: "ls".into(),
            focused: true,
            ..Default::default()
        };
        let cells = bar.cells(40, 3);
        // prompt + text present
        assert!(cells.iter().any(|c| c.c == '>'));
        assert!(cells.iter().any(|c| c.c == 'l'));
        assert!(cells.iter().any(|c| c.c == 's'));
        // the '>' prompt is accent-green when focused
        let prompt = cells.iter().find(|c| c.c == '>').unwrap();
        assert_eq!(prompt.fg, ACCENT);
        // a block cursor is shown while focused
        assert!(cells.iter().any(|c| c.c == '█'));
    }

    #[test]
    fn cells_long_text_follows_cursor_tail() {
        let text = format!("S{}E", "x".repeat(80));
        let bar = InputBar {
            text,
            focused: true,
            ..Default::default()
        };
        let cells = bar.cells(20, 3);
        // the tail (end of input) and the cursor are visible…
        assert!(cells.iter().any(|c| c.c == 'E'));
        assert!(cells.iter().any(|c| c.c == '█'));
        // …while the start has scrolled out of view
        assert!(!cells.iter().any(|c| c.c == 'S'));
    }

    #[test]
    fn cells_shows_dim_ghost_suggestion() {
        let bar = InputBar {
            text: "/se".into(),
            focused: true,
            ..Default::default()
        };
        let cells = bar.cells(40, 3);
        // the completion "ttings" is shown as a dim ghost, with no block cursor
        assert!(cells.iter().any(|c| c.c == 't' && c.fg == DIM));
        assert!(!cells.iter().any(|c| c.c == '█'));
    }

    #[test]
    fn cells_unfocused_has_no_cursor() {
        let bar = InputBar {
            text: "ls".into(),
            focused: false,
            ..Default::default()
        };
        assert!(!bar.cells(40, 3).iter().any(|c| c.c == '█'));
    }

    #[test]
    fn cells_unfocused_prompt_is_dim() {
        let bar = InputBar {
            text: String::new(),
            focused: false,
            ..Default::default()
        };
        let prompt = bar.cells(40, 3).into_iter().find(|c| c.c == '>').unwrap();
        assert_eq!(prompt.fg, DIM);
    }

    #[test]
    fn cells_tiny_returns_empty() {
        assert!(InputBar::default().cells(3, 3).is_empty());
        assert!(InputBar::default().cells(40, 0).is_empty());
    }
}

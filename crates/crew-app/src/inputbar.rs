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
        let max = cols.saturating_sub(start + 1) as usize;
        let text_len = self.text.chars().count();
        let mut full = format!("> {}", self.text);
        if self.focused {
            full.push('█'); // block cursor while typing, like a terminal
        }
        let display: String = full.chars().take(max).collect();
        let cursor_idx = 2 + text_len; // position of the block cursor in `full`
        let mut out = Vec::new();
        for (i, ch) in display.chars().enumerate() {
            let fg = if i < 2 {
                prompt_fg
            } else if self.focused && i == cursor_idx {
                ACCENT
            } else {
                TEXT_FG
            };
            out.push(CellView {
                col: start + i as u16,
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
        let (ch, enter, backspace) = match &key.logical_key {
            Key::Named(NamedKey::Enter) => (None, true, false),
            Key::Named(NamedKey::Backspace) => (None, false, true),
            Key::Named(NamedKey::Space) => (Some(' '), false, false),
            Key::Character(s) => (s.chars().next(), false, false),
            _ => (None, false, false),
        };
        crate::chatlayout::input_reduce(&mut self.text, ch, enter, backspace)
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
    fn cells_unfocused_has_no_cursor() {
        let bar = InputBar {
            text: "ls".into(),
            focused: false,
        };
        assert!(!bar.cells(40, 3).iter().any(|c| c.c == '█'));
    }

    #[test]
    fn cells_unfocused_prompt_is_dim() {
        let bar = InputBar {
            text: String::new(),
            focused: false,
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

//! Docked bottom command bar: a single-line text input rendered as a titled card.
use crew_render::CellView;

use crate::boxdraw::{self, BoxRect};

const BG: (u8, u8, u8) = (8, 8, 16);
const BORDER_DIM: (u8, u8, u8) = (70, 130, 140);
const TITLE_DIM: (u8, u8, u8) = (200, 200, 200);
const ACCENT: (u8, u8, u8) = (0, 255, 160);
const TEXT_FG: (u8, u8, u8) = (220, 220, 220);

#[derive(Default)]
pub struct InputBar {
    pub text: String,
    pub focused: bool,
}

impl InputBar {
    /// Render the input card as a grid of `cols × rows` cells.
    pub fn cells(&self, cols: u16, rows: u16) -> Vec<CellView> {
        if cols < 6 || rows < 3 {
            return Vec::new();
        }
        let right = cols.saturating_sub(1);
        let bottom = rows.saturating_sub(1);
        let (border, title_fg) = if self.focused {
            (ACCENT, ACCENT)
        } else {
            (BORDER_DIM, TITLE_DIM)
        };
        let mut out = boxdraw::titled_box(
            BoxRect {
                left: 0,
                top: 0,
                right,
                bottom,
            },
            "INPUT",
            border,
            title_fg,
            BG,
        );
        // Content row 1, starting at col 2: "> " then text, truncated to fit.
        let content_col: u16 = 2;
        let max_content = right.saturating_sub(content_col + 1) as usize;
        let prompt = "> ";
        let display: String = format!("{}{}", prompt, self.text)
            .chars()
            .take(max_content)
            .collect();
        for (i, ch) in display.chars().enumerate() {
            out.push(CellView {
                col: content_col + i as u16,
                row: 1,
                c: ch,
                fg: TEXT_FG,
                bg: BG,
                bold: false,
                italic: false,
            });
        }
        out
    }

    /// If `text` is non-empty, take it and return `Some`; otherwise `None`.
    pub fn submit(&mut self) -> Option<String> {
        if self.text.is_empty() {
            None
        } else {
            Some(std::mem::take(&mut self.text))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cells_focused_contains_corners_and_text() {
        let bar = InputBar {
            text: "ls".into(),
            focused: true,
        };
        let cells = bar.cells(40, 3);
        let has = |ch: char| cells.iter().any(|c| c.c == ch);
        assert!(has('╭'), "missing top-left corner");
        assert!(has('╯'), "missing bottom-right corner");
        assert!(has('>'), "missing prompt '>'");
        assert!(has('l'), "missing 'l'");
        assert!(has('s'), "missing 's'");
    }

    #[test]
    fn submit_returns_text_and_clears() {
        let mut bar = InputBar {
            text: "hi".into(),
            focused: false,
        };
        let result = bar.submit();
        assert_eq!(result, Some("hi".to_string()));
        assert!(bar.text.is_empty());
    }

    #[test]
    fn submit_empty_returns_none() {
        let mut bar = InputBar::default();
        assert_eq!(bar.submit(), None);
    }

    #[test]
    fn cells_tiny_returns_empty() {
        let bar = InputBar::default();
        assert!(bar.cells(5, 3).is_empty());
        assert!(bar.cells(40, 2).is_empty());
    }
}

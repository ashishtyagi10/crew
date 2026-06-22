//! Docked bottom command bar: a single-line text input. The surrounding pane
//! draws the rounded border (so it bottom-aligns with the sidebar/panes); this
//! only renders the `> text` content inside it.
use crew_render::CellView;

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
}

#[cfg(test)]
#[path = "inputbar_tests.rs"]
mod tests;

//! Docked bottom command bar: a single-line text input drawn as a rounded
//! fieldset card. The working directory rides the top border as the card's
//! legend (`╭─ ~/code/crew ─╮`); the `> text` prompt sits on the interior row.
use std::path::PathBuf;

use crew_render::CellView;

use crate::boxdraw::titled_card;

const BG: (u8, u8, u8) = (0, 0, 0);
const ACCENT: (u8, u8, u8) = (0, 255, 160);
const DIM: (u8, u8, u8) = (120, 130, 140);
const TEXT_FG: (u8, u8, u8) = (220, 220, 220);
const BROADCAST: (u8, u8, u8) = (220, 120, 200);
/// Border colour of the card: bright when focused, muted grey otherwise.
const BORDER_ON: (u8, u8, u8) = (210, 210, 220);
const BORDER_OFF: (u8, u8, u8) = (110, 110, 120);
/// Transient status message colour (amber), shown on the bottom border.
const STATUS_FG: (u8, u8, u8) = (230, 180, 90);
/// Faint placeholder hint shown in an empty, focused input bar.
const PLACEHOLDER: (u8, u8, u8) = (90, 95, 105);
const PLACEHOLDER_TEXT: &str = "type / for commands";

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
    /// Whether broadcast (synchronized input to all panes) is active.
    pub broadcast: bool,
    /// Crew's working directory: rendered (`~`-abbreviated) as the bar's legend
    /// and used as the base for `cd` directory completion. Empty = none.
    pub cwd: PathBuf,
}

impl InputBar {
    /// The ghost-suffix to show after the typed text (and insert on Tab/→): the
    /// highlighted palette command, else `cd` directory completion, else a
    /// history/slash autosuggestion. `None` when unfocused or nothing completes.
    pub(crate) fn ghost(&self) -> Option<String> {
        if !self.focused {
            return None;
        }
        let m = crate::suggest::matches(&self.text);
        if !m.is_empty() {
            let name = m[self.menu_sel.min(m.len() - 1)].name;
            return Some(name[self.text.len()..].to_string());
        }
        if self.text.starts_with("cd ") && !self.cwd.as_os_str().is_empty() {
            return crate::suggest::dir_suggest(&self.text, &self.cwd);
        }
        crate::suggest::suggest(&self.text, &self.history)
    }
}

impl InputBar {
    /// Render the input card: a rounded border with the working directory as its
    /// top-border legend, `> text` on the interior row, and an optional transient
    /// `status` message on the bottom border. Prompt and border brighten on focus.
    pub fn cells(&self, cols: u16, rows: u16, status: Option<&str>) -> Vec<CellView> {
        if cols < 6 || rows < 3 {
            return Vec::new();
        }
        // Interior row between the top (legend) and bottom borders.
        let row = rows / 2;
        // The card frame with the cwd riding the top border as its legend.
        let legend = if self.cwd.as_os_str().is_empty() {
            String::new()
        } else {
            crate::cwd::display(&self.cwd)
        };
        let border = if self.focused { BORDER_ON } else { BORDER_OFF };
        let mut out = titled_card(cols, rows, &legend, border, ACCENT, BG);

        // A distinct magenta "» " prompt signals broadcast (input → all panes).
        let (prompt, base) = if self.broadcast {
            ("» ", BROADCAST)
        } else {
            ("> ", ACCENT)
        };
        let prompt_fg = if self.focused { base } else { DIM };
        // Prompt starts inside the left border (col 0); text follows the prompt.
        let pstart = 2u16;
        let tstart = pstart + 2;
        // Keep text clear of the right border at `cols - 1`.
        let text_area = (cols.saturating_sub(tstart + 1)) as usize;
        // Typed text (bright), then either the ghost suggestion (dim) or the
        // block cursor when there's nothing to suggest.
        let mut body: Vec<(char, (u8, u8, u8))> = self.text.chars().map(|c| (c, TEXT_FG)).collect();
        match &self.ghost() {
            Some(g) => body.extend(g.chars().map(|c| (c, DIM))),
            None if self.focused => body.push(('█', ACCENT)),
            None => {}
        }
        // Follow the cursor: when the body overflows the field, show its tail.
        let skip = body.len().saturating_sub(text_area);
        for (i, ch) in prompt.chars().enumerate() {
            out.push(cell(pstart + i as u16, row, ch, prompt_fg));
        }
        for (i, &(ch, fg)) in body[skip..].iter().enumerate() {
            out.push(cell(tstart + i as u16, row, ch, fg));
        }

        // Faint placeholder past the cursor when the bar is empty and focused.
        if self.text.is_empty() && self.focused {
            for (i, ch) in PLACEHOLDER_TEXT.chars().enumerate() {
                let col = tstart + 2 + i as u16;
                if col >= cols - 1 {
                    break;
                }
                out.push(cell(col, row, ch, PLACEHOLDER));
            }
        }

        // Transient status flashed on the bottom border, right-aligned.
        if let Some(s) = status {
            let label = format!(" {s} ");
            let w = label.chars().count() as u16;
            if w + 3 < cols {
                let start = cols - 2 - w;
                for (i, ch) in label.chars().enumerate() {
                    out.push(cell(start + i as u16, rows - 1, ch, STATUS_FG));
                }
            }
        }
        out
    }
}

fn cell(col: u16, row: u16, c: char, fg: (u8, u8, u8)) -> CellView {
    CellView {
        col,
        row,
        c,
        fg,
        bg: BG,
        bold: false,
        italic: false,
    }
}

#[cfg(test)]
#[path = "inputbar_tests.rs"]
mod tests;

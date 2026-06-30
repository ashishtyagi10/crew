//! Docked bottom command bar: a single-line text input drawn as a rounded
//! fieldset card. The working directory rides the top border as the card's
//! legend (`╭─ ~/code/crew ─╮`); the `> text` prompt sits on the interior row.
use std::cell::RefCell;
use std::path::PathBuf;

use crew_render::CellView;

use crate::boxdraw::titled_card;

use crate::palette::accent;
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
    /// Text typed before history navigation began; Up/Down recall only entries
    /// starting with it (empty = match everything, i.e. plain recall).
    pub hist_prefix: String,
    /// Whether broadcast (synchronized input to all panes) is active.
    pub broadcast: bool,
    /// Crew's working directory: rendered (`~`-abbreviated) as the bar's legend
    /// and used as the base for `cd` directory completion. Empty = none.
    pub cwd: PathBuf,
    /// Memoized `ghost()` result. `ghost()` runs on every render frame, and for
    /// `cd`/`/edit`/`/open` it does a `read_dir`; without this cache a path
    /// partial sitting in the bar re-scans the directory on every redraw (e.g.
    /// ~15×/s while a pane animates). Interior mutability so the cache fills from
    /// the `&self` render path. Keyed on the inputs `ghost()` actually depends on.
    pub(crate) ghost_cache: RefCell<GhostCache>,
}

/// Cached `ghost()` output and the `(text, menu_sel, cwd)` it was computed for.
#[derive(Default)]
pub(crate) struct GhostCache {
    key: Option<(String, usize, PathBuf)>,
    val: Option<String>,
}

impl InputBar {
    /// The ghost-suffix to show after the typed text (and insert on Tab/→): the
    /// highlighted palette command, else `cd` directory completion, else a
    /// history/slash autosuggestion. `None` when unfocused or nothing completes.
    ///
    /// Memoized: `compute_ghost` can hit the filesystem, but this runs every
    /// frame, so a result is reused until the typed text, palette selection, or
    /// working directory changes.
    pub(crate) fn ghost(&self) -> Option<String> {
        let key = (self.text.clone(), self.menu_sel, self.cwd.clone());
        {
            let cache = self.ghost_cache.borrow();
            if cache.key.as_ref() == Some(&key) {
                return cache.val.clone();
            }
        }
        let val = self.compute_ghost();
        *self.ghost_cache.borrow_mut() = GhostCache {
            key: Some(key),
            val: val.clone(),
        };
        val
    }

    /// Uncached `ghost()` computation — see [`InputBar::ghost`].
    fn compute_ghost(&self) -> Option<String> {
        if !self.focused {
            return None;
        }
        let m = crate::suggest::matches(&self.text);
        if !m.is_empty() {
            let name = m[self.menu_sel.min(m.len() - 1)].name;
            // Only the highlighted command extends inline as ghost text; a fuzzy
            // (non-prefix) match shows no suffix but the palette still lists it
            // and Tab/Enter fills the full name.
            return name.strip_prefix(self.text.as_str()).map(str::to_string);
        }
        if !self.cwd.as_os_str().is_empty() {
            if self.text.starts_with("cd ") {
                return crate::suggest::dir_suggest(&self.text, &self.cwd);
            }
            // `/edit`/`/open` complete file and directory paths.
            if let Some(p) = crate::pathcomplete::path_suggest(&self.text, &self.cwd) {
                return Some(p);
            }
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
            // Keep the tail (current dir) when the path is deeper than the card.
            crate::cwd::fit_legend(
                &crate::cwd::display(&self.cwd),
                cols.saturating_sub(6) as usize,
            )
        };
        let border = if self.focused {
            crew_theme::theme().border_focused
        } else {
            crew_theme::theme().border_normal
        };
        let mut out = titled_card(
            cols,
            rows,
            &legend,
            border,
            accent(),
            crew_theme::theme().page_bg,
        );

        // A distinct magenta "» " prompt signals broadcast (input → all panes).
        let (prompt, base) = if self.broadcast {
            ("» ", crew_theme::theme().broadcast)
        } else {
            ("> ", accent())
        };
        let prompt_fg = if self.focused {
            base
        } else {
            crew_theme::theme().dim
        };
        // Prompt starts inside the left border (col 0); text follows the prompt.
        let pstart = 2u16;
        let tstart = pstart + 2;
        // Keep text clear of the right border at `cols - 1`.
        let text_area = (cols.saturating_sub(tstart + 1)) as usize;
        // Typed text (bright), then either the ghost suggestion (dim) or the
        // block cursor when there's nothing to suggest.
        let mut body: Vec<(char, (u8, u8, u8))> = self
            .text
            .chars()
            .map(|c| (c, crew_theme::theme().ink))
            .collect();
        match &self.ghost() {
            Some(g) => body.extend(g.chars().map(|c| (c, crew_theme::theme().dim))),
            None if self.focused => body.push(('█', accent())),
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
                out.push(cell(col, row, ch, crew_theme::theme().placeholder));
            }
        }

        // Transient status flashed on the bottom border, right-aligned.
        if let Some(s) = status {
            let label = format!(" {s} ");
            let w = label.chars().count() as u16;
            if w + 3 < cols {
                let start = cols - 2 - w;
                for (i, ch) in label.chars().enumerate() {
                    out.push(cell(
                        start + i as u16,
                        rows - 1,
                        ch,
                        crew_theme::theme().status_fg,
                    ));
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
        bg: crew_theme::theme().page_bg,
        bold: false,
        italic: false,
    }
}

#[cfg(test)]
#[path = "inputbar_tests.rs"]
mod tests;

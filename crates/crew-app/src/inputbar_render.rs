//! Rendering for the docked input bar card (state lives in `inputbar`).
//! Text placement is width-aware (see `chatwidth`): emoji/CJK in the typed
//! text advance two columns, so glyphs never overlap and the caret and
//! right-aligned status stay put.
use crew_render::CellView;

use crate::chatwidth::{char_w, clip_w, place_row, str_w};
use crate::inputbar::InputBar;
use crate::palette::accent;

const PLACEHOLDER_TEXT: &str = "type / for commands";

impl InputBar {
    /// Render the input card: a rounded border with the working directory as its
    /// top-border legend, `> text` on the interior row, and an optional transient
    /// `status` message on the bottom border. Prompt and border brighten on focus.
    ///
    /// `pane` is the focused pane's name, drawn as a right-aligned legend on the
    /// bottom border — the mirror of the cwd riding the top one. Typing here acts
    /// on whichever pane is selected, and until now the bar never said which.
    pub fn cells(
        &self,
        cols: u16,
        rows: u16,
        status: Option<&str>,
        pane: Option<&str>,
    ) -> Vec<CellView> {
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
                crate::boxdraw::title_budget(cols),
            )
        };
        // Focus mode is a MODE: the one thing a mode owes the user is a
        // standing sign that it is on, or they will spend the afternoon
        // wondering why nothing pops. The bar's legend is the only chrome
        // always on screen, and the word rides in front of the path.
        let legend = if crate::focusmode::on() {
            let budget = crate::boxdraw::title_budget(cols);
            let tag = "\u{25c9} focus";
            if legend.is_empty() {
                tag.to_string()
            } else {
                let room = budget.saturating_sub(tag.chars().count() + 3);
                format!("{tag} \u{b7} {}", crate::cwd::fit_legend(&legend, room))
            }
        } else {
            legend
        };
        let border = if self.focused {
            crew_theme::theme().border_focused
        } else {
            crew_theme::theme().border_normal
        };
        let mut out = crate::modernring::gradient_card(
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
        // Follow the cursor: when the body overflows the field, show its tail
        // (measured in display columns — wide glyphs count two).
        let mut total: usize = body.iter().map(|&(c, _)| char_w(c)).sum();
        let mut skip = 0;
        while total > text_area && skip < body.len() {
            total -= char_w(body[skip].0);
            skip += 1;
        }
        place_row(
            pstart,
            cols,
            prompt.chars().map(|c| (c, prompt_fg)),
            |x, ch, fg| {
                out.push(cell(x, row, ch, fg));
            },
        );
        place_row(
            tstart,
            cols - 1,
            body[skip..].iter().copied(),
            |x, ch, fg| {
                out.push(cell(x, row, ch, fg));
            },
        );

        // Faint placeholder past the cursor when the bar is empty and focused.
        if self.text.is_empty() && self.focused {
            let ph = crew_theme::theme().placeholder;
            place_row(
                tstart + 2,
                cols - 1,
                PLACEHOLDER_TEXT.chars().map(|c| (c, ph)),
                |x, ch, fg| out.push(cell(x, row, ch, fg)),
            );
        }

        // Transient status flashed on the bottom border, right-aligned — and
        // when nothing is flashing, the focused pane's name in the same spot.
        // A status is a moment; the pane legend is the bar's standing answer to
        // "where does this go?", so the flash borrows the slot and gives it back.
        let bottom = match status {
            Some(s) => Some((format!(" {s} "), crew_theme::theme().status_fg)),
            None => pane
                .map(str::trim)
                .filter(|n| !n.is_empty())
                .map(|n| (format!(" {n} "), crew_theme::theme().legend_off)),
        };
        if let Some((label, fg)) = bottom {
            // Clip to the columns the bottom rule actually owns, so a long pane
            // title can never overrun the `╯` corner.
            let label = clip_w(&label, cols.saturating_sub(4) as usize);
            let w = str_w(&label) as u16;
            if w + 3 < cols {
                place_row(
                    cols - 2 - w,
                    cols,
                    label.chars().map(|c| (c, fg)),
                    |x, ch, fg| out.push(cell(x, rows - 1, ch, fg)),
                );
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
        ..Default::default()
    }
}

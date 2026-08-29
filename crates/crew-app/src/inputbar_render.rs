//! Rendering for the docked input bar card (state lives in `inputbar`).
//! Text placement is width-aware (see `chatwidth`): emoji/CJK in the typed
//! text advance two columns, so glyphs never overlap and the caret and
//! right-aligned status stay put.
use crew_render::CellView;

use crate::chatwidth::{char_w, place_row, str_w};
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
        pending: Option<&str>,
        status: Option<&str>,
        pane: Option<&str>,
    ) -> Vec<CellView> {
        if cols < 6 || rows < 3 {
            return Vec::new();
        }
        // Interior row between the top (legend) and bottom borders.
        let row = rows / 2;
        // Where you are in history, at the right end of the TOP rule — the
        // one border slot the bar never used (see `inputlegend::history_tag`).
        // Measured before the legend is fitted, so a deep path gives way to it
        // rather than being silently overwritten by it.
        let history =
            crate::inputlegend::history_tag(&self.history, &self.hist_prefix, self.hist_pos, cols);
        let reserved = history.as_ref().map_or(0, |(l, _)| str_w(l) + 1);

        // The card frame with the cwd riding the top border as its legend
        // (and the focus-mode tag in front of it) — see `inputlegend`.
        let legend = crate::inputlegend::top(&self.cwd, cols, reserved);
        let border = if self.focused {
            crate::panecardglow::focused_stroke(crew_theme::theme())
        } else {
            crew_theme::theme().border_normal
        };
        // The legend follows focus with everything else. It is the loudest
        // thing on the card, and while it stayed full accent on a blurred bar
        // the brightest mark on screen belonged to the surface you had just
        // left — the border dimmed, the prompt dimmed, the path did not.
        let legend_fg = if self.focused {
            accent()
        } else {
            crew_theme::theme().legend_off
        };
        let mut out = crate::modernring::gradient_card(
            cols,
            rows,
            &legend,
            border,
            legend_fg,
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
        // What you have typed, coloured by what it MEANS: a command that
        // resolves, one still being typed, one that never will, plus flags
        // and quoted runs (see `inputink`).
        let mut body: Vec<(char, (u8, u8, u8))> = self
            .text
            .chars()
            .zip(crate::inputink::paint(&self.text))
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
        // A line longer than the field scrolls to follow the caret, and until
        // now it did so in silence: the bar showed the tail of your command
        // with nothing saying the head existed. The prompt's own gutter — the
        // blank column between `>` and the text — carries the mark.
        let gutter = if skip > 0 {
            ('\u{2026}', crew_theme::theme().dim)
        } else {
            (' ', prompt_fg)
        };
        let mut prompt_cells = prompt.chars().map(|c| (c, prompt_fg)).collect::<Vec<_>>();
        if let Some(last) = prompt_cells.last_mut() {
            *last = gutter;
        }
        place_row(pstart, cols, prompt_cells, |x, ch, fg| {
            out.push(cell(x, row, ch, fg));
        });
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

        // Where you are in history, at the right end of the TOP rule — the
        // one border slot the bar never used (see `inputlegend::history_tag`).
        let history =
            crate::inputlegend::history_tag(&self.history, &self.hist_prefix, self.hist_pos, cols);
        if let Some((label, fg)) = history {
            place_tag(&mut out, &label, fg, cols, 0);
        }

        // The bottom rule's tag: a flashing status, else the focused pane's
        // name, both budgeted so the rule stays a rule (see `inputlegend`).
        let bottom = crate::inputlegend::bottom(pending, status, pane, cols);
        if let Some((label, fg)) = bottom {
            place_tag(&mut out, &label, fg, cols, rows - 1);
        }
        out
    }
}

/// Right-align `label` on one of the card's border rows, clear of the corner.
fn place_tag(out: &mut Vec<CellView>, label: &str, fg: (u8, u8, u8), cols: u16, row: u16) {
    let w = str_w(label) as u16;
    if w + 3 >= cols {
        return;
    }
    place_row(
        cols - 2 - w,
        cols,
        label.chars().map(|c| (c, fg)),
        |x, ch, fg| out.push(cell(x, row, ch, fg)),
    );
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

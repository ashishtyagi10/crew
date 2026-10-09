//! The masked key prompt as a card — the drawing half of [`super`], split out
//! when the hint became a ladder and the file crossed its cap.
//!
//! `card` is a fieldset with the variable in its legend, one interior row of
//! mask glyphs, and (when there is one) the hint row under it. The hint is
//! picked at the width the pane ALLOWED, not the width the card asked for:
//! `keyhint::want` asks for room for the longest form and `popupplace` may
//! clamp it, and a hint cut in half is worse than a shorter one whole.
use crew_render::CellView;

use crate::popupplace::Popup;

/// The prompt as a composer pop-up: a fieldset card with the variable
/// named in the legend, one interior row of mask glyphs (one per typed
/// character, clipped to the card), as wide as `keyhint::want` says in
/// a pane `cols` wide — it grows with the pasted key.
impl super::KeyEntry {
    /// The prompt as a composer pop-up.
    pub(crate) fn card(&self, cols: u16) -> Popup {
        let t = crew_theme::theme();
        let bg = t.page_bg;
        let put = |col: u16, row: u16, c: char, fg: (u8, u8, u8), bold: bool| CellView {
            col,
            row,
            c,
            fg,
            bg,
            bold,
            italic: false,
            ..Default::default()
        };
        let typed = self.buf.chars().count();
        let cols = crate::popupplace::card_cols(crate::keyhint::want(self.hint(), typed), cols);
        // Lowercase, matching every other composer-overlay legend. The legend
        // is never a leak risk: the secret is drawn on its own interior row
        // (row 1) and the leak test scopes its assertion to that row alone.
        // `fit_legend` keeps the tail (the variable name) over the head (the
        // word "paste") when the card is too narrow for both.
        let title = crate::cwd::fit_legend(
            &format!("paste {}", self.var),
            crate::boxdraw::title_budget(cols),
        );
        let rows = self.rows();
        let mut cells = crate::popupchrome::card(cols, rows, &title);
        if cells.is_empty() {
            return Popup { cells, cols, rows };
        }
        // A `❯` at the field's head, as on the composer: this row is typed
        // into. The mask and the hint start one column past it.
        let inner = cols.saturating_sub(4) as usize;
        cells.push(put(
            1,
            1,
            crate::glyphs::prompt(),
            crate::palette::accent(),
            true,
        ));
        // A key longer than the field ends in a muted `…`: the dots used to
        // just stop at the edge, 28 of a pasted 40, as if that were all.
        let over = typed > inner;
        let dots = if over { inner.saturating_sub(1) } else { typed };
        cells.extend((0..dots).map(|i| put(3 + i as u16, 1, '•', t.ink, false)));
        if over {
            cells.push(put(3 + dots as u16, 1, '\u{2026}', t.text_muted, false));
        }
        // The longest form that fits the card the pane allowed (`keyhint`),
        // which is not always the one the card asked room for.
        if let Some(hint) = crate::keyhint::fitting(&self.var, self.waiting, inner) {
            // ROW 2 IS LOAD-BEARING, not decoration: the hint text contains
            // almost every character of a typical key, so drawing it on row 1
            // would make the leak assertion (which scopes itself to row 1)
            // vacuous. A test pins it here.
            let row = crate::chatwidth::clip_w(hint, inner);
            cells.extend(
                row.chars()
                    .enumerate()
                    .map(|(i, c)| put(3 + i as u16, 2, c, t.text_muted, false)),
            );
        }
        Popup { cells, cols, rows }
    }
}

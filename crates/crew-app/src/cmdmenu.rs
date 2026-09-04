//! Command palette: the slash commands matching the current input, rendered as
//! the interior of a fieldset "commands" card on the canvas (the border + legend
//! are drawn by [`crate::panelcard::push_card`]). Just a box on the one canvas,
//! like every other panel — no opaque floating popup.
use crew_render::CellView;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{List, ListItem, ListState, StatefulWidget};

use crate::suggest::MenuItem;

/// Most command rows shown at once; beyond this the palette scrolls to keep the
/// selection in view (the list grew past a comfortable popup height).
const MAX_ROWS: usize = 10;

/// Never narrower than this, so a one-word list is still a card and not a
/// sliver floating in the middle of a pane.
const MIN_ROW_W: u16 = 24;

/// Total cell rows the "commands" card needs for `n` commands: the visible list
/// rows (capped at [`MAX_ROWS`]) plus the top/bottom fieldset border. The caller
/// sizes the card with this; [`crate::panelcard::push_card`] insets the 2 border
/// rows back out before asking [`menu_cells`] to fill the interior.
pub fn menu_rows(n: usize) -> u16 {
    n.min(MAX_ROWS) as u16 + 2
}

/// Build a `title`-legended fieldset card (`cols × rows`): the dim border +
/// legend framing the item list ("commands" for the palette, "files" for the
/// chat @file popup). Rendered as a single overlay scene so the overlay pass
/// backs it with solid black — a box on the canvas, fully opaque.
pub fn menu_card(
    title: &str,
    matches: &[MenuItem],
    sel: usize,
    cols: u16,
    rows: u16,
) -> Vec<CellView> {
    if cols < 4 || rows < 3 || matches.is_empty() {
        return Vec::new();
    }
    let t = crew_theme::theme();
    let mut cells = crate::modernring::gradient_card(
        cols,
        rows,
        title,
        t.border_normal,
        t.legend_off,
        t.page_bg,
    );
    // The list fills the 1-cell-inset interior; shift it inside the border.
    for mut cell in menu_cells(matches, sel, cols - 2, rows - 2) {
        cell.col += 1;
        cell.row += 1;
        cells.push(cell);
    }

    cells
}

/// Render the command list into the card's `cols × rows` interior. Every cell is
/// transparent over the card's black backdrop — the selected row is marked by the
/// `›` symbol and bold text, never a background bar (a bar washed out the dim
/// description text).
pub(crate) fn menu_cells(matches: &[MenuItem], sel: usize, cols: u16, rows: u16) -> Vec<CellView> {
    if cols < 2 || rows < 1 || matches.is_empty() {
        return Vec::new();
    }
    // Laid out at the width the rows need, and CENTRED in what is left: the
    // chord right-aligns to the row's edge, so a row as wide as the pane put
    // it a screen away from its own command (see `cmdrow::content_w`).
    let w = (crate::cmdrow::content_w(matches) as u16).clamp(MIN_ROW_W.min(cols), cols);
    let pad = (cols - w) / 2;
    let mut buf = Buffer::empty(Rect::new(0, 0, w, rows));
    // Two columns of the row go to the selection marker; every row is laid out
    // in what is left, so the description column and the chord agree with the
    // width the list actually draws in.
    let avail = usize::from(w).saturating_sub(2);
    let label_w = crate::cmdrow::label_col(matches, avail);
    let swatch_w = crate::cmdrow::swatch_col(matches);
    let dim = crate::menuink::desc_color();
    let items: Vec<ListItem> = matches
        .iter()
        .map(|c| ListItem::new(crate::cmdrow::spans(c, label_w, swatch_w, avail, dim)))
        .collect();
    let list = List::new(items)
        // No background bar — bold the selected row so its text stays fully legible.
        .highlight_style(Style::new().add_modifier(Modifier::BOLD))
        .highlight_symbol("› ");
    let mut state = ListState::default();
    // A list of nothing but notes and titles has no row to mark.
    state.select(crate::cmdnote::selectable(matches).then(|| sel.min(matches.len() - 1)));
    StatefulWidget::render(list, buf.area, &mut buf, &mut state);
    crate::tui::to_cells(&buf)
        .into_iter()
        .map(|mut c| {
            c.col += pad;
            c
        })
        .collect()
}

#[cfg(test)]
#[path = "cmdmenu_tests.rs"]
mod tests;

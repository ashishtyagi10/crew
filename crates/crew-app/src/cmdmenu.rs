//! Command palette: the slash commands matching the current input, rendered as
//! the interior of a fieldset "commands" card on the canvas (the border + legend
//! are drawn by [`crate::panelcard::push_card`]). Just a box on the one canvas,
//! like every other panel — no opaque floating popup.
use crew_render::CellView;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState, StatefulWidget};

use crate::boxdraw::titled_card;
use crate::suggest::MenuItem;

use crate::palette::accent_color;
const DIM: Color = Color::Rgb(120, 130, 140);

/// Most command rows shown at once; beyond this the palette scrolls to keep the
/// selection in view (the list grew past a comfortable popup height).
const MAX_ROWS: usize = 10;

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
    let mut cells = titled_card(cols, rows, title, t.border_normal, t.legend_off, t.page_bg);
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
fn menu_cells(matches: &[MenuItem], sel: usize, cols: u16, rows: u16) -> Vec<CellView> {
    if cols < 2 || rows < 1 || matches.is_empty() {
        return Vec::new();
    }
    let mut buf = Buffer::empty(Rect::new(0, 0, cols, rows));
    let items: Vec<ListItem> = matches
        .iter()
        .map(|c| {
            if c.header {
                // A section title, not a choice: dim + bold, no desc column.
                return ListItem::new(Line::from(Span::styled(
                    c.label.clone(),
                    Style::new().fg(DIM).add_modifier(Modifier::BOLD),
                )));
            }
            ListItem::new(Line::from(vec![
                Span::styled(c.label.clone(), Style::new().fg(accent_color())),
                Span::raw("  "),
                Span::styled(c.desc.clone(), Style::new().fg(DIM)),
            ]))
        })
        .collect();
    let list = List::new(items)
        // No background bar — bold the selected row so its text stays fully legible.
        .highlight_style(Style::new().add_modifier(Modifier::BOLD))
        .highlight_symbol("› ");
    let mut state = ListState::default();
    state.select(Some(sel.min(matches.len() - 1)));
    StatefulWidget::render(list, buf.area, &mut buf, &mut state);
    crate::tui::to_cells(&buf)
}

#[cfg(test)]
#[path = "cmdmenu_tests.rs"]
mod tests;

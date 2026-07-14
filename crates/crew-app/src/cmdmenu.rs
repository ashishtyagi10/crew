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
mod tests {
    use super::*;

    #[test]
    fn card_legend_is_the_given_title() {
        let matches = crate::suggest::menu_items("/s");
        let cells = menu_card("files", &matches, 0, 40, menu_rows(matches.len()));
        // The legend on the top border spells the title.
        let row0: String = {
            let mut cs: Vec<_> = cells.iter().filter(|c| c.row == 0).collect();
            cs.sort_by_key(|c| c.col);
            cs.iter().map(|c| c.c).collect()
        };
        assert!(row0.contains("files"));
    }

    #[test]
    fn card_has_fieldset_border_legend_and_command_text() {
        let matches = crate::suggest::menu_items("/s");
        assert!(matches.len() >= 2); // /settings, /shell
        let cells = menu_card("commands", &matches, 0, 40, menu_rows(matches.len()));
        assert!(cells.iter().any(|c| c.c == '╭')); // fieldset corner
        assert!(cells.iter().any(|c| c.c == 'c')); // "commands" legend / text
        assert!(cells.iter().any(|c| c.c == 's')); // command text present
        assert!(cells.iter().any(|c| c.c == '›')); // selection marker present
    }

    #[test]
    fn card_bg_uniform_no_highlight_bar() {
        let matches = crate::suggest::menu_items("/s");
        let cells = menu_card("commands", &matches, 0, 40, menu_rows(matches.len()));
        // No selection bar that could wash out text: every cell background is
        // uniform (the theme page_bg), so the description stays legible on any row.
        let bg = crew_theme::theme().page_bg;
        assert!(
            cells.iter().all(|c| c.bg == bg),
            "menu background must be uniform (no highlight bar)"
        );
    }

    #[test]
    fn selected_row_is_bold_and_marked() {
        let matches = crate::suggest::menu_items("/"); // every command
        let cells = menu_card("commands", &matches, 0, 40, menu_rows(matches.len()));
        // Selected row is interior row 0 → card row 1: marked by `›`, and its
        // glyphs are bold (the only visual cue, never an obscuring background).
        assert!(cells.iter().any(|c| c.c == '›' && c.row == 1));
        assert!(cells.iter().any(|c| c.row == 1 && c.bold));
        // A non-selected row (card row 2) is not bold.
        assert!(cells.iter().filter(|c| c.row == 2).all(|c| !c.bold));
    }

    #[test]
    fn empty_matches_render_nothing() {
        assert!(menu_card("commands", &[], 0, 40, 5).is_empty());
        assert!(menu_cells(&[], 0, 40, 5).is_empty());
    }

    #[test]
    fn menu_rows_caps_long_lists() {
        assert_eq!(menu_rows(3), 5); // short list: exact fit
        assert_eq!(menu_rows(50), MAX_ROWS as u16 + 2); // long list: capped
    }

    #[test]
    fn long_list_scrolls_to_selection() {
        let all = crate::suggest::menu_items("/"); // every command
        assert!(all.len() > MAX_ROWS, "need a list longer than the cap");
        let rows = menu_rows(all.len());
        assert_eq!(rows as usize, MAX_ROWS + 2); // height is capped
                                                 // selecting the last command still renders it (the list scrolled): the
                                                 // selection marker is drawn within the capped popup.
        let cells = menu_cells(&all, all.len() - 1, 40, rows);
        assert!(cells.iter().any(|c| c.c == '›'));
    }
}

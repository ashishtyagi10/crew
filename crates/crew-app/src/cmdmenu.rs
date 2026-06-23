//! Command palette: a rounded popup listing the slash commands that match the
//! current input, rendered with ratatui and converted to `CellView`s. Drawn as
//! an overlay just above the input bar.
use crew_render::CellView;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, List, ListItem, ListState, StatefulWidget};

use crate::suggest::Cmd;

const ACCENT: Color = Color::Rgb(0, 255, 160);
const DIM: Color = Color::Rgb(120, 130, 140);
/// Slightly raised tint so the popup is opaque over panes behind it (a cell
/// only draws a background quad when its bg differs from the default).
const MENU_BG: Color = Color::Rgb(18, 18, 30);

/// Most command rows shown at once; beyond this the palette scrolls to keep the
/// selection in view (the list grew past a comfortable popup height).
const MAX_ROWS: usize = 10;

/// Number of cell rows a menu of `n` commands needs (list + top/bottom border),
/// capped at [`MAX_ROWS`] visible rows so a long list doesn't overflow upward.
pub fn menu_rows(n: usize) -> u16 {
    n.min(MAX_ROWS) as u16 + 2
}

/// Render the command palette into a `cols × rows` grid.
pub fn menu_cells(matches: &[&Cmd], sel: usize, cols: u16, rows: u16) -> Vec<CellView> {
    if cols < 6 || rows < 3 || matches.is_empty() {
        return Vec::new();
    }
    let mut buf = Buffer::empty(Rect::new(0, 0, cols, rows));
    let items: Vec<ListItem> = matches
        .iter()
        .map(|c| {
            ListItem::new(Line::from(vec![
                Span::styled(c.name, Style::new().fg(ACCENT)),
                Span::raw("  "),
                Span::styled(c.desc, Style::new().fg(DIM)),
            ]))
        })
        .collect();
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(ACCENT))
        .style(Style::new().bg(MENU_BG))
        .title(Span::styled(" commands ", Style::new().fg(ACCENT)));
    let list = List::new(items)
        .block(block)
        // Muted selection tint — keeps the accent name + dim desc readable.
        .highlight_style(Style::new().bg(Color::Rgb(45, 55, 75)))
        .highlight_symbol("› ");
    let mut state = ListState::default();
    state.select(Some(sel.min(matches.len() - 1)));
    StatefulWidget::render(list, buf.area, &mut buf, &mut state);
    crate::tui::to_cells_opaque(&buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_matching_commands_with_border() {
        let matches = crate::suggest::matches("/s");
        assert!(matches.len() >= 2); // /settings, /shell
        let cells = menu_cells(&matches, 0, 40, menu_rows(matches.len()));
        assert!(cells.iter().any(|c| c.c == '╭'));
        assert!(cells.iter().any(|c| c.c == 's')); // command text present
    }

    #[test]
    fn empty_matches_render_nothing() {
        assert!(menu_cells(&[], 0, 40, 5).is_empty());
    }

    #[test]
    fn menu_rows_caps_long_lists() {
        assert_eq!(menu_rows(3), 5); // short list: exact fit
        assert_eq!(menu_rows(50), MAX_ROWS as u16 + 2); // long list: capped
    }

    #[test]
    fn long_list_scrolls_to_selection() {
        let all = crate::suggest::matches("/"); // every command
        assert!(all.len() > MAX_ROWS, "need a list longer than the cap");
        let rows = menu_rows(all.len());
        assert_eq!(rows as usize, MAX_ROWS + 2); // height is capped
                                                 // selecting the last command still renders it (the list scrolled): the
                                                 // selection marker is drawn within the capped popup.
        let cells = menu_cells(&all, all.len() - 1, 40, rows);
        assert!(cells.iter().any(|c| c.c == '›'));
    }
}

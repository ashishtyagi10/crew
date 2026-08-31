use super::*;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, BorderType, Widget};

#[test]
fn rounded_block_yields_corner_cells() {
    let mut buf = Buffer::empty(Rect::new(0, 0, 10, 3));
    Block::bordered()
        .border_type(BorderType::Rounded)
        .render(buf.area, &mut buf);
    let cells = to_cells(&buf);
    assert!(cells.iter().any(|c| c.c == '╭'));
    assert!(cells.iter().any(|c| c.c == '╯'));
}

#[test]
fn blank_buffer_yields_no_cells() {
    let buf = Buffer::empty(Rect::new(0, 0, 8, 2));
    assert!(to_cells(&buf).is_empty());
}

#[test]
fn opaque_fills_blank_bg_cells_with_blocks() {
    use ratatui::style::{Color, Style};
    let _g = crate::app::theme_test_guard();
    // A popup's own sheet colour, which is the PAGE colour: the in-pane
    // path has nothing to say about it (there is nothing behind an
    // in-pane surface to bleed through), the overlay path fills it.
    let page = crew_theme::theme().page_bg;
    let mut buf = Buffer::empty(Rect::new(0, 0, 6, 2));
    buf.set_style(
        buf.area,
        Style::new().bg(Color::Rgb(page.0, page.1, page.2)),
    );
    assert!(to_cells(&buf).is_empty());
    let cells = to_cells_opaque(&buf);
    assert_eq!(cells.len(), 12);
    assert!(cells.iter().all(|c| c.c == '█' && c.fg == page));
}

/// A blank cell carrying a background that is not the page's own is a
/// highlight — a cursor bar, a selected row, an inverted header — and
/// dropping it took the middle out of every one of them.
#[test]
fn a_highlight_survives_its_own_spaces() {
    use ratatui::style::{Color, Style};
    let _g = crate::app::theme_test_guard();
    let page = crew_theme::theme().page_bg;
    let mut buf = Buffer::empty(Rect::new(0, 0, 7, 1));
    // "a b" on a bar: the space between the glyphs is part of the bar.
    let bar = Style::new()
        .fg(Color::Rgb(0, 0, 0))
        .bg(Color::Rgb(0, 200, 120));
    buf.set_string(0, 0, "a b", bar);
    let cells = to_cells(&buf);
    let bar_cells: Vec<char> = (0..3)
        .filter_map(|x| cells.iter().find(|c| c.col == x).map(|c| c.c))
        .collect();
    assert_eq!(bar_cells.len(), 3, "the bar came apart: {bar_cells:?}");
    assert_eq!(bar_cells[1], '█', "the space is filled, not dropped");

    // A blank at the PAGE colour is still nothing worth drawing.
    let mut plain = Buffer::empty(Rect::new(0, 0, 4, 1));
    plain.set_style(
        plain.area,
        Style::new().bg(Color::Rgb(page.0, page.1, page.2)),
    );
    assert!(
        to_cells(&plain).is_empty(),
        "a page-coloured blank is not a highlight"
    );
}

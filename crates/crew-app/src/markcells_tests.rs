//! Every place crew turns text into cells keeps a combining mark on the cell
//! before it (`CellView::marks`) — a virama, a nasal dot — instead of
//! dropping it or giving it a column, and copying the cells gives it back.
use crew_render::CellView;

/// `(col, char, marks)` of each cell, in column order.
fn shape(mut cells: Vec<CellView>) -> Vec<(u16, char, String)> {
    cells.sort_by_key(|c| (c.row, c.col));
    cells
        .iter()
        .filter(|c| c.c != ' ')
        .map(|c| (c.col, c.c, c.marks.chars().collect()))
        .collect()
}

/// `क्ष हिंदी` as every producer should place it.
fn want(at: u16) -> Vec<(u16, char, String)> {
    [
        (0, 'क', "\u{94d}"),
        (1, 'ष', ""),
        (3, 'ह', ""),
        (4, '\u{93f}', "\u{902}"),
        (5, 'द', ""),
        (6, '\u{940}', ""),
    ]
    .iter()
    .map(|&(c, ch, m)| (c + at, ch, m.to_string()))
    .collect()
}

const TEXT: &str = "क्ष हिंदी";

#[test]
fn place_row_and_chat_lines_keep_marks_on_their_letters() {
    let mut out = Vec::new();
    let end =
        crate::chatwidth::place_row(2, 40, TEXT.chars().map(|c| (c, ())), |col, c, marks, ()| {
            out.push(CellView {
                col,
                c,
                marks,
                ..Default::default()
            })
        });
    assert_eq!(shape(out), want(2));
    assert_eq!(end, 2 + 7, "marks take no column");
    let line: crate::chatbody::CardLine = TEXT
        .chars()
        .map(|c| crate::chatbody::plain(c, (0, 0, 0), false))
        .collect();
    assert_eq!(
        shape(crate::chatplace::line_cells(0, &line, 40, (9, 9, 9))),
        want(0)
    );
}

#[test]
fn ratatui_graphemes_and_inbox_rows_become_cells_with_marks() {
    let area = ratatui::layout::Rect::new(0, 0, 20, 1);
    let mut buf = ratatui::buffer::Buffer::empty(area);
    buf.set_string(0, 0, TEXT, ratatui::style::Style::default());
    assert_eq!(shape(crate::tui::to_cells(&buf)), want(0));
    let mut line = crate::inbox::line::Line::new(20);
    line.put(0, 20, TEXT, (0, 0, 0), false);
    let mut cells = Vec::new();
    line.emit(&mut cells, 0, None);
    assert_eq!(shape(cells), want(0));
}

#[test]
fn a_selection_copies_its_marks() {
    let mut cells = Vec::new();
    crate::chatwidth::place_row(0, 40, TEXT.chars().map(|c| (c, ())), |col, c, marks, ()| {
        cells.push(CellView {
            col,
            c,
            marks,
            ..Default::default()
        })
    });
    let sel = crate::gridsel::CellSel {
        pane: 0,
        anchor: (0, 0),
        cursor: (39, 0),
    };
    assert_eq!(crate::gridsel::selection_text(&cells, &sel), TEXT);
}

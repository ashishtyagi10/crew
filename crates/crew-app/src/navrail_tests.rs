use super::*;

fn pane(index: usize) -> PaneRow {
    PaneRow {
        index,
        title: format!("pane {index}"),
        focused: false,
        activity: false,
        minimized: false,
        attention: None,
        busy: false,
        unread: 0,
        hovered: false,
    }
}

/// The glyph drawn at `(col, row)`, if anything was.
fn at(v: &[CellView], col: u16, row: u16) -> Option<char> {
    v.iter().find(|c| c.col == col && c.row == row).map(|c| c.c)
}

#[test]
fn the_rail_is_seven_columns_at_every_cell_width() {
    // The card's own column count is `floor(w / cw)`; a rail that lands a
    // rounding short of seven has nowhere to put the mark.
    for cw in [6.0, 7.5, 8.0, 9.3, 11.0, 14.0, 21.7] {
        let cols = (px(cw) / cw).floor() as u16;
        assert_eq!(cols, RAIL_COLS, "cw {cw} gave {cols} columns");
    }
}

#[test]
fn the_chevron_points_the_way_the_edge_will_move() {
    assert_eq!(chevron(true), '\u{203a}', "collapsed opens to the right");
    assert_eq!(chevron(false), '\u{2039}', "open puts it away to the left");
    let open = legend(false, "crew v9.9.9");
    assert!(open.starts_with(chevron(false)), "{open}");
    assert!(
        open.contains("crew v9.9.9"),
        "the legend kept its name: {open}"
    );
    let shut = legend(true, "crew v9.9.9");
    assert_eq!(
        shut,
        chevron(true).to_string(),
        "the rail's legend IS the chevron"
    );
}

#[test]
fn the_chevron_target_is_three_cells_of_the_legend_row_and_not_the_corner() {
    let card = Rect {
        x: 100.0,
        y: 40.0,
        w: 70.0,
        h: 600.0,
    };
    let r = chevron_rect(card, 10.0, 20.0);
    assert_eq!(
        (r.x, r.y),
        (120.0, 40.0),
        "centred on the legend's first cell"
    );
    assert_eq!((r.w, r.h), (30.0, 20.0), "three cells of the legend row");
    assert!(
        !crate::chrome::point_in(r, 105.0, 50.0),
        "the corner is not the button"
    );
    let chev_x = card.x + f32::from(CHEVRON_COL) * 10.0 + 5.0;
    assert!(crate::chrome::point_in(r, chev_x, 50.0), "the chevron is");
    assert!(
        !crate::chrome::point_in(r, 125.0, 70.0),
        "the row below the legend is not the button"
    );
}

#[test]
fn row_one_is_the_first_pane_and_the_border_is_not() {
    assert_eq!(pane_at_row(0, 10, 3), None, "the top border is not a pane");
    assert_eq!(pane_at_row(1, 10, 3), Some(0));
    assert_eq!(pane_at_row(3, 10, 3), Some(2));
    assert_eq!(pane_at_row(4, 10, 3), None, "past the last pane");
    assert_eq!(pane_at_row(3, 2, 9), None, "past the rows the card has");
}

#[test]
fn a_rail_row_says_which_pane_has_focus_and_what_each_is_doing() {
    let _g = crate::app::theme_test_guard();
    let mut rows = [pane(1), pane(2), pane(3)];
    rows[0].focused = true;
    rows[1].busy = true;
    rows[2].attention = Some(('!', true));
    let v = rail_cells(&rows, 5, 10, '+');
    assert_eq!(at(&v, 0, 0), Some('\u{25b8}'), "the focused row is marked");
    assert_eq!(at(&v, 1, 0), Some('1'));
    assert_eq!(at(&v, 0, 1), Some(' '), "an unfocused row has no caret");
    assert_eq!(
        at(&v, 4, 1),
        Some('+'),
        "the busy pane spins in the mark column"
    );
    assert_eq!(at(&v, 4, 2), Some('!'), "the raised attention is its glyph");
    assert_eq!(at(&v, 4, 0), None, "a quiet pane marks nothing");
    let bell = crew_theme::theme().bell;
    let marked = v.iter().find(|c| c.col == 4 && c.row == 2).unwrap();
    assert_eq!(
        marked.fg, bell,
        "the mark wears the same colour the full row does"
    );
}

#[test]
fn the_mark_beats_the_number_for_the_last_two_columns() {
    // Five inner columns and a crew that has run to four digits: the number
    // is what gets cut, never the thing that says the pane needs you. (The
    // mark is pushed after the head, so a head that reached its column would
    // still be the cell found there.)
    let mut rows = [pane(1234)];
    rows[0].attention = Some(('\u{2691}', true));
    let v = rail_cells(&rows, 5, 10, '+');
    assert_eq!(
        at(&v, 4, 0),
        Some('\u{2691}'),
        "the number took the mark's column"
    );
    assert!(
        !at(&v, 3, 0).is_some_and(|c| c.is_ascii_digit()),
        "no column of air between the number and the mark"
    );
    // A number that does fit is not cut, and still clears the mark.
    let two = rail_cells(&[pane(12)], 5, 10, '+');
    assert_eq!((at(&two, 1, 0), at(&two, 2, 0)), (Some('1'), Some('2')));
    assert_eq!(at(&two, 3, 0), None);
}

#[test]
fn a_blinking_marker_is_absent_on_its_dark_phase_and_the_row_stays() {
    let mut rows = [pane(1)];
    rows[0].attention = Some(('!', false));
    let v = rail_cells(&rows, 5, 10, '+');
    assert_eq!(at(&v, 4, 0), None, "mid-blink the marker is not drawn");
    assert_eq!(
        at(&v, 1, 0),
        Some('1'),
        "the row itself does not blink away"
    );
}

#[test]
fn the_rail_draws_only_the_rows_the_card_has() {
    let rows: Vec<PaneRow> = (1..=9).map(pane).collect();
    let v = rail_cells(&rows, 5, 3, '+');
    assert!(
        v.iter().all(|c| c.row < 3),
        "a pane was drawn past the card's last row"
    );
    assert!(v.iter().any(|c| c.row == 2), "and the last row was used");
}
#[test]
fn the_whole_rail_card_reads_as_one_column() {
    let _g = crate::app::theme_test_guard();
    let t = crew_theme::theme();
    let mut rows = [pane(1), pane(2), pane(3)];
    rows[1].focused = true;
    rows[2].attention = Some(('!', true));
    let (cols, irows) = (RAIL_COLS - 2, 6u16);
    let card = crate::modernring::gradient_card(
        RAIL_COLS,
        irows + 2,
        &legend(true, ""),
        t.border_normal,
        t.legend_off,
        t.page_bg,
    );
    // The chevron sits where a legend starts: past the corner and its rule
    // cell, inside the click target `chevron_rect` claims.
    assert_eq!(
        at(&card, CHEVRON_COL, 0),
        Some(chevron(true)),
        "the legend IS the button, and it is where the target looks"
    );
    // And the rail's own cells, offset by the border, never land on it.
    let inner = rail_cells(&rows, cols, irows, '+');
    assert!(
        inner.iter().all(|c| c.col < cols && c.row < irows),
        "a rail cell would be drawn over the card's frame"
    );
    let drawn: Vec<(u16, u16)> = inner.iter().map(|c| (c.col + 1, c.row + 1)).collect();
    assert!(
        drawn
            .iter()
            .all(|&(col, row)| col > 0 && col < RAIL_COLS - 1 && row > 0),
        "the rail overlaps its own border: {drawn:?}"
    );
}

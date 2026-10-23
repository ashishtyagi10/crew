//! A panel row: its size stops a column short of the border, and under the
//! cursor its name is bold on the fill.

/// In both panels, the cell just left of every border on a file's row is
/// blank: `3.7K│` read as part of the line.
#[test]
fn a_size_keeps_a_column_of_air_before_the_border() {
    let base = std::env::temp_dir().join("crew_far_rowair");
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    std::fs::write(base.join("readme.md"), vec![b'x'; 3_800]).unwrap();
    let pane = crate::farpane::FarPane::new(base);
    let cells = {
        let _lock = crate::app::THEME_LOCK.lock();
        crate::farpane::render::render(&pane, 80, 24)
    };
    let at = |row, col| {
        cells
            .iter()
            .find(|c| c.row == row && c.col == col)
            .map(|c| c.c)
    };
    let line = |row| -> String {
        let mut v: Vec<_> = cells.iter().filter(|c| c.row == row).collect();
        v.sort_by_key(|c| c.col);
        v.iter().map(|c| c.c).collect()
    };
    let row = (0..24)
        .find(|&r| line(r).contains("readme"))
        .expect("the file's row");
    let bars: Vec<u16> = cells
        .iter()
        .filter(|c| c.row == row && c.c == '\u{2502}')
        .map(|c| c.col)
        .collect();
    assert_eq!(
        bars.len(),
        3,
        "two panels' borders and the divider: {bars:?}"
    );
    for col in bars.into_iter().skip(1) {
        let before = at(row, col - 1).unwrap_or(' ');
        assert_eq!(before, ' ', "a size touches the border at column {col}");
    }
}

/// The cursor bar's name is bold on the accent fill, like the path tab above
/// it: a regular-weight `../` vanished into a phosphor tube's glow.
#[test]
fn the_cursor_bar_is_bold_on_its_fill() {
    let base = std::env::temp_dir().join("crew_far_boldbar");
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(base.join("alpha")).unwrap();
    let pane = crate::farpane::FarPane::new(base);
    let _g = crate::app::theme_test_guard();
    let page = crew_theme::theme().page_bg;
    let cells = crate::farpane::render::render(&pane, 80, 24);
    // Row 1 is the first listing row of both panels: `../`, under the cursor.
    let bar: Vec<_> = cells
        .iter()
        .filter(|c| c.row == 1 && c.bg != page && c.c != ' ')
        .collect();
    assert!(bar.iter().any(|c| c.c == '.'), "the bar holds ../");
    assert!(
        bar.iter().all(|c| c.bold),
        "every inked cell on the bar is bold"
    );
}

/// The rows under the panels — the selection's path, `crew $`, the F-key
/// bar — start on the listing's column, inside the frames' left rule: from
/// the rule's own cell they stood 3px left of the stroke above them.
#[test]
fn the_rows_under_the_panels_start_inside_the_frames_rule() {
    let pane = crate::farpane::FarPane::new(std::env::temp_dir());
    let cells = {
        let _lock = crate::app::THEME_LOCK.lock();
        crate::farpane::render::render(&pane, 80, 24)
    };
    let first = |row: u16| {
        let mut v: Vec<_> = cells
            .iter()
            .filter(|c| c.row == row && c.c != ' ')
            .collect();
        v.sort_by_key(|c| c.col);
        v.first().map(|c| (c.col, c.c))
    };
    assert_eq!(
        first(0).map(|f| f.0),
        Some(0),
        "the frame's rule is column 0"
    );
    for row in 21..24 {
        assert_eq!(
            first(row).map(|f| f.0),
            Some(1),
            "row {row}: {:?}",
            first(row)
        );
    }
}

/// The passive panel's frame is drawn in the frame role, like every other
/// quiet frame, not in the muted text ink (survey I5).
#[test]
fn a_passive_panels_frame_is_a_frame_not_text() {
    let _g = crate::app::theme_test_guard();
    let base = std::env::temp_dir().join("crew_far_passive_edge");
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    let pane = crate::farpane::FarPane::new(base);
    let border = crew_theme::theme().border_normal;
    let cells = crate::farpane::render::render(&pane, 80, 24);
    let right = cells
        .iter()
        .filter(|c| c.c == '\u{2502}')
        .max_by_key(|c| c.col)
        .expect("the passive panel's right edge");
    assert_eq!(right.fg, border, "edge at column {}", right.col);
}

/// The line between a blurred pane's two lists is frame, as its edges are:
/// in the muted text ink it stood 7–10:1 in a box drawn at 2:1.
#[test]
fn a_blurred_panes_divider_is_frame_too() {
    let _g = crate::app::theme_test_guard();
    let base = std::env::temp_dir().join("crew_far_passive_divider");
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    let pane = crate::farpane::FarPane::new(base);
    let cells = crate::farpane::render::render_in(&pane, 80, 24, false);
    let top = cells.iter().find(|c| c.c == '\u{252c}').expect("the ┬");
    let column: Vec<_> = cells.iter().filter(|c| c.col == top.col).collect();
    assert!(column.len() > 3, "the divider runs down");
    let border = crew_theme::theme().border_normal;
    for c in column
        .iter()
        .filter(|c| "\u{252c}\u{2502}\u{2534}".contains(c.c))
    {
        assert_eq!(c.fg, border, "{} at row {}", c.c, c.row);
    }
}

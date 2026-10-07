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

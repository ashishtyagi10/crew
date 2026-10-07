//! A panel row's size stops a column short of the border after it.

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

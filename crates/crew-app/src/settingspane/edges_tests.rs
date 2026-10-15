use crate::config::CrewConfig;
use crate::settingspane::SettingsPane;
use crew_render::CellView;

/// The fg of the frame strokes on `row`.
fn strokes(cells: &[CellView], row: u16) -> Vec<(u16, (u8, u8, u8))> {
    let mut v: Vec<(u16, (u8, u8, u8))> = cells
        .iter()
        .filter(|c| c.row == row && c.c == '\u{2502}')
        .map(|c| (c.col, c.fg))
        .collect();
    v.sort_unstable();
    v
}

/// Squared distance from the page, summed over the channels.
fn from_page(c: (u8, u8, u8)) -> i32 {
    let p = crew_theme::theme().page_bg;
    let d = |a: u8, b: u8| (i32::from(a) - i32::from(b)).pow(2);
    d(c.0, p.0) + d(c.1, p.1) + d(c.2, p.2)
}

/// Where the form runs on below the viewport, the strokes on its last row
/// lean toward the page — the same row, a few rows above the cut, wears
/// them at full strength. Cut square, the frames ended in open strokes
/// (glass survey E12).
#[test]
fn the_rows_at_a_cut_fade_toward_the_page() {
    let _g = crate::app::theme_test_guard();
    let p = SettingsPane::new(CrewConfig::default(), Vec::new());
    let rows = 14;
    // The viewport's last row is `rows - 3`; four rows taller, it is inside.
    let cut = strokes(&p.cells(80, rows), rows - 3);
    let whole = strokes(&p.cells(80, rows + 4), rows - 3);
    assert!(!cut.is_empty(), "a frame crosses the cut");
    assert_eq!(cut.len(), whole.len());
    for ((col, faded), (_, full)) in cut.iter().zip(&whole) {
        assert!(
            from_page(*faded) < from_page(*full),
            "col {col}: {faded:?} vs {full:?}"
        );
    }
}

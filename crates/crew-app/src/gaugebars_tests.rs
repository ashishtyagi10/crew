//! Narrow SYSTEM bars are capsules where the glyphs were, and only there.
use super::*;

fn stats() -> Stats {
    Stats {
        cpu: 0.2,
        mem: 0.7,
        disk: 0.95,
        ..Default::default()
    }
}

#[test]
fn a_narrow_nav_draws_three_capsules_in_the_bar_columns() {
    let _g = crate::app::theme_test_guard();
    let cols = 20;
    assert!(!crate::sysdials::fits(cols));
    let p = paint(crate::sysdials::NAV.rows, stats(), cols, 5, 2.0);
    assert!(!p.is_empty());
    for r in &p {
        assert!(r.x >= f32::from(BAR_COL) - 0.01, "{r:?}");
        assert!(r.x + r.w <= f32::from(cols - TAIL) + 0.01, "{r:?}");
        assert!((5.0..8.0).contains(&r.y), "rows 5..8: {r:?}");
    }
    // The load tier's colour, per reading: the critical disk is not the cpu.
    assert!(p.iter().any(|r| r.color == fill_color(0.95)));
    assert!(p.iter().any(|r| r.color == fill_color(0.2)));
}

#[test]
fn nothing_when_the_dials_fit_or_for_the_dash() {
    let _g = crate::app::theme_test_guard();
    assert!(paint(crate::sysdials::NAV.rows, stats(), 60, 5, 2.0).is_empty());
    assert!(paint(crate::sysdials::DASH.rows, stats(), 20, 5, 2.0).is_empty());
}

/// The cell pass leaves the bar blank for the capsule to fill.
#[test]
fn the_cells_leave_the_bar_to_the_capsule() {
    let _g = crate::app::theme_test_guard();
    let cells = crate::gauges::render_stats(stats(), 20, 12, None);
    assert!(!cells.iter().any(|c| c.c == '\u{2588}' || c.c == '\u{2591}'));
    let row: String = cells.iter().filter(|c| c.row == 1).map(|c| c.c).collect();
    assert!(row.contains("CPU") && row.contains("20%"), "{row:?}");
}

//! The drawn panes' `put` helpers mark a cut the way every card legend
//! does. `/dash`, `/usage` and `/disk` each had a private copy that broke at
//! the edge by char count — no `…`, no display width — while
//! [`crate::chatwidth::clip_w`] is "the one clip used everywhere on the
//! canvas". They route through [`crate::navtext::put_at`] now.
use crew_render::CellView;

fn text(cells: &[CellView], row: u16) -> String {
    let mut on: Vec<&CellView> = cells.iter().filter(|c| c.row == row).collect();
    on.sort_by_key(|c| c.col);
    on.iter().map(|c| c.c).collect()
}

#[test]
fn dash_put_marks_its_cut_and_stops_before_the_edge() {
    let _g = crate::app::theme_test_guard();
    let mut out = Vec::new();
    crate::dashlayout::put(&mut out, "a long host name and more", 1, 0, (1, 2, 3), 12);
    let s = text(&out, 0);
    assert!(s.ends_with('\u{2026}'), "{s:?}");
    assert!(out.iter().all(|c| c.col < 12), "{s:?}");
    let mut out = Vec::new();
    crate::dashlayout::put(&mut out, "fits", 1, 0, (1, 2, 3), 12);
    assert_eq!(text(&out, 0), "fits");
}

#[test]
fn disk_put_marks_its_cut() {
    let _g = crate::app::theme_test_guard();
    let mut out = Vec::new();
    crate::disktile::put(&mut out, "node_modules/some/deep/path", 2, 3, (1, 2, 3), 14);
    let s = text(&out, 3);
    assert!(s.ends_with('\u{2026}'), "{s:?}");
    assert!(out.iter().all(|c| c.col < 14), "{s:?}");
}

/// `/usage`'s totals line at the pane's narrowest: cut with a mark, and one
/// column of air kept at the right edge as before.
#[test]
fn usage_totals_line_marks_its_cut_at_the_narrowest_pane() {
    let _g = crate::app::theme_test_guard();
    use crate::usageledger::{DAYS, HOURS};
    let b = crate::usageledger::Buckets {
        hourly: vec![0u64; DAYS * HOURS],
        daily_cost: vec![0; DAYS],
        tok_in: 1_840_000,
        tok_out: 410_000,
        cost_microusd: 1_980_000,
    };
    let cells = crate::usagepane::cells(&b, 24, 12);
    let s = text(&cells, 1);
    assert!(s.ends_with('\u{2026}'), "{s:?}");
    assert!(
        cells.iter().filter(|c| c.row == 1).all(|c| c.col < 23),
        "{s:?}"
    );
}

//! A failed plan's banner.
use super::failed_banner;

/// `plan failed:` is the danger ink, bold, where `planning:` is plain — and
/// the last row says how to leave.
#[test]
fn a_failed_plan_says_so_and_how_to_leave() {
    let _g = crate::app::theme_test_guard();
    let danger = crew_theme::readable::danger(crew_theme::theme());
    let cells = failed_banner("no planner model is configured", 40, 6);
    // By column: blank cells are not emitted.
    let row = |r: u16| -> String {
        let mut line = vec![' '; 40];
        for c in cells.iter().filter(|c| c.row == r) {
            line[usize::from(c.col)] = c.c;
        }
        line.into_iter().collect()
    };
    assert!(row(0).starts_with("plan failed:"), "{:?}", row(0));
    let lead: Vec<_> = cells.iter().filter(|c| c.row == 0 && c.col < 4).collect();
    assert!(lead.iter().all(|c| c.fg == danger && c.bold));
    assert_eq!(row(5).trim(), "Esc closes it");
    for rows in 0..3 {
        let _ = failed_banner("x", 20, rows); // too short for the hint: no panic
    }
}

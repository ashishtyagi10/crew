//! The welcome names its build once.
use super::*;

/// The build is named once on the welcome: by the news line when it fits,
/// else by the stamp in the corner — never by both.
#[test]
fn the_version_is_said_once() {
    let _g = crate::app::theme_test_guard();
    let ver = env!("CARGO_PKG_VERSION");
    for (cols, rows) in [(80u16, 24u16), (40, 24), (140, 40)] {
        let cells = welcome_cells_animated(cols, rows, 0, None);
        let said = (0..rows)
            .filter(|&r| {
                let mut row: Vec<_> = cells.iter().filter(|c| c.row == r).collect();
                row.sort_by_key(|c| c.col);
                row.iter().map(|c| c.c).collect::<String>().contains(ver)
            })
            .count();
        assert_eq!(said, 1, "{cols}x{rows}: the version on {said} rows");
    }
}

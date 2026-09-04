//! The rows `/keys` owed and did not list — split from `help_tests` for the
//! line cap.
use crate::helptable::BINDINGS;

/// The bar's own keys, and the chord that was in neither list. `Cmd+O`
/// opened a pane (a dead one, on a binary nothing builds) and was documented
/// nowhere; `Cmd+]` / `Cmd+[` were in the manual and allowlisted OUT of the
/// overlay; the bar's Tab and history recall were listed only as composer
/// keys, so `/keys` implied they did nothing in the bar.
#[test]
fn the_bar_keys_and_the_lost_chords_are_listed() {
    for needle in [
        "Cmd+O",
        "Cmd+] / Cmd+[",
        "Tab / \u{2192} (in input)",
        "\u{2191} / \u{2193} (in input)",
    ] {
        assert!(
            BINDINGS.iter().any(|(k, _)| *k == needle),
            "/keys lacks `{needle}`"
        );
    }
}

/// The bottom hint on a narrow overlay is cut with a mark, not started at
/// column zero with its tail dropped off the right edge.
#[test]
fn the_hint_marks_its_cut_on_a_narrow_overlay() {
    let _g = crate::app::theme_test_guard();
    let cells = crate::help::help_cells(30, 12, 0, "");
    let last = cells.iter().map(|c| c.row).max().expect("rows");
    let hint: String = cells
        .iter()
        .filter(|c| c.row == last)
        .map(|c| c.c)
        .collect();
    assert!(hint.contains('\u{2026}'), "{hint:?}");
    assert!(cells.iter().all(|c| c.col < 30), "nothing past the edge");
}

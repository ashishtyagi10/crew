use super::{card, mark};

/// The pop-up's frame is the FOCUSED stroke, not the quiet one a waiting
/// pane wears — it is the box the next key lands in.
#[test]
fn a_popup_wears_the_focused_stroke_and_a_bold_accent_legend() {
    let _g = crate::app::theme_test_guard();
    let t = crew_theme::theme();
    let quiet = crate::modernring::gradient_card(
        40,
        5,
        "commands",
        t.border_normal,
        t.legend_off,
        t.page_bg,
    );
    let focused = card(40, 5, "commands");
    let corner =
        |v: &[crew_render::CellView]| v.iter().find(|c| c.row == 4 && c.col == 0).map(|c| c.fg);
    assert_ne!(corner(&focused), corner(&quiet), "the stroke changed");
    let legend: Vec<_> = focused
        .iter()
        .filter(|c| c.row == 0 && c.c.is_alphabetic())
        .collect();
    assert_eq!(legend.len(), "commands".len());
    assert!(legend
        .iter()
        .all(|c| c.bold && c.fg == crate::palette::accent()));
}

/// The mark sits on the top border, two columns in from the right corner,
/// and is dropped when there is no room to keep frame either side of it.
#[test]
fn a_mark_rides_the_top_border_at_the_right() {
    let _g = crate::app::theme_test_guard();
    let mut v = card(40, 5, "commands");
    let n = v.len();
    mark(&mut v, "13/14", 40);
    let text: String = v[n..].iter().map(|c| c.c).collect();
    assert_eq!(text, "13/14");
    assert!(v[n..].iter().all(|c| c.row == 0), "on the top border");
    assert_eq!(v[n].col, 40 - 2 - 5, "two columns in from the corner");
    let before = v.len();
    mark(&mut v, "13/14", 8);
    assert_eq!(v.len(), before, "no room, no mark");
}

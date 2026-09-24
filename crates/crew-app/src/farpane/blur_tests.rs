//! A `/far` pane that is not focused: no caret, and no accent cursor bar.
use crate::farpane::FarPane;

fn pane() -> FarPane {
    let base = std::env::temp_dir().join("crew_far_render_blur");
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(base.join("alpha")).unwrap();
    FarPane::new(base)
}

#[test]
fn a_blurred_pane_has_no_caret_and_no_accent_bar() {
    let _g = crate::app::theme_test_guard();
    let p = pane();
    let acc = crate::palette::accent();
    let live = super::render_in(&p, 80, 24, true);
    let quiet = super::render_in(&p, 80, 24, false);
    assert!(live.iter().any(|c| c.c == '\u{258f}'), "focused: caret");
    assert!(live.iter().any(|c| c.bg == acc), "focused: accent bar");
    assert!(
        !quiet.iter().any(|c| c.c == '\u{258f}'),
        "blurred: no caret"
    );
    assert!(
        !quiet.iter().any(|c| c.bg == acc),
        "blurred: no accent fill"
    );
    let wash = crew_theme::readable::selection_bg(&crew_theme::theme());
    assert!(
        quiet.iter().any(|c| c.bg == wash),
        "blurred: the place is kept, in the wash"
    );
}

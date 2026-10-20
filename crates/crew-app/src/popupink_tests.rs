//! A pop-up wears the focused frame of the page it opens on: the heavy line
//! on an Ink page, the light one on glass.
use crew_theme::ThemeId;

fn strokes(id: ThemeId) -> (usize, usize) {
    crew_theme::set_theme(id);
    let cells = super::card(20, 6, "projects");
    let edge = |c: &&crew_render::CellView| c.row == 0 || c.row == 5 || c.col == 0 || c.col == 19;
    let count = |set: &[char]| {
        cells
            .iter()
            .filter(edge)
            .filter(|c| set.contains(&c.c))
            .count()
    };
    (
        count(&['\u{2501}', '\u{2503}']),
        count(&['\u{2500}', '\u{2502}']),
    )
}

#[test]
fn on_an_ink_page_a_pop_up_is_the_heavy_frame() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    for id in [ThemeId::PaperLight, ThemeId::PaperDark] {
        let (heavy, light) = strokes(id);
        assert!(
            heavy > 0 && light == 0,
            "{id:?}: {heavy} heavy, {light} light"
        );
    }
}

#[test]
fn on_glass_a_pop_up_keeps_the_light_frame() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    let (heavy, light) = strokes(ThemeId::Nebula);
    assert!(heavy == 0 && light > 0, "{heavy} heavy, {light} light");
}

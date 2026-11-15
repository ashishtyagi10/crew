use super::*;

/// What the canvas drew under `(x, y)`, in square units: its colour and
/// alpha, or nothing.
fn at(c: &Canvas, x: f32, y: f32) -> ((u8, u8, u8), f32) {
    let u = c.row_units();
    c.paint()
        .iter()
        .find(|p| p.x <= x && x < p.x + p.w && p.y * u <= y && y < (p.y + p.h) * u)
        .map_or(((0, 0, 0), 0.0), |p| (p.color, p.alpha))
}

/// On a pale soft page the face is the page's colour, the rim toward the
/// light is brightened and the rim away from it is darkened: a knob.
#[test]
fn the_face_is_the_page_and_the_two_rims_say_where_it_ends() {
    let _g = crate::app::theme_test_guard();
    crew_theme::set_theme(crew_theme::ThemeId::Porcelain);
    let page = crew_theme::theme().page_bg;
    let mut c = Canvas::with_sub(12, 6, 2.0, 8);
    let (centre, r) = ((6.0, 6.0), 4.0);
    mould(&mut c, centre, r);
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
    assert_eq!(at(&c, 6.0, 6.0), (page, 1.0), "the face");
    // Just outside the face, up-left (toward the light) and down-right.
    let k = (r + 0.15) / (0.45_f32).hypot(0.89);
    // Laid over the page, the rim toward the light is brighter than the
    // page and the rim away from it darker.
    let over = |(rgb, a): ((u8, u8, u8), f32)| {
        let mix = |c: u8, p: u8| f32::from(c) * a + f32::from(p) * (1.0 - a);
        mix(rgb.0, page.0) + mix(rgb.1, page.1) + mix(rgb.2, page.2)
    };
    let flat = over((page, 1.0));
    let lit = over(at(&c, 6.0 - 0.45 * k, 6.0 - 0.89 * k));
    let dark = over(at(&c, 6.0 + 0.45 * k, 6.0 + 0.89 * k));
    assert!(lit > flat + 6.0, "lit rim {lit} on a page of {flat}");
    assert!(dark < flat - 6.0, "far rim {dark} on a page of {flat}");
    // Well past the shadows nothing is drawn.
    assert_eq!(at(&c, 0.2, 0.2).1, 0.0);
}

use super::wear_for;

/// Every theme's bar is its page, solid, with the page's ink: the user,
/// 2026-10-18, "in the glass theme the title bar is still transparent … make
/// it like it is in light and dark". An Ink Reader page paints its bar too:
/// on a dark Mac the native one was a dark strip across the white page. The
/// ink is the page's too: a dark Mac drew a white title on a light bar.
#[test]
fn every_title_bar_is_its_page_solid() {
    for id in crew_theme::ALL_THEMES {
        let t = id.theme();
        let (r, g, b) = t.page_bg;
        assert_eq!(wear_for(t), Some(([r, g, b], 1.0, t.dark)), "{id:?}");
    }
}

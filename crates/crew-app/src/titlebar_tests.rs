use super::{wanted, wear_for};

#[test]
fn a_solid_window_leaves_the_native_title_bar_alone() {
    assert_eq!(wanted(1.0, (1, 2, 3), false, None), None);
}

#[test]
fn a_sheer_window_paints_its_title_bar_the_page_colour() {
    assert_eq!(
        wanted(0.88, (1, 2, 3), true, None),
        Some(([1, 2, 3], 1.0, true))
    );
    assert_eq!(
        wanted(0.25, (9, 9, 9), true, None),
        Some(([9, 9, 9], 1.0, true))
    );
}

/// A light page gets a dark title whatever the OS is in: on a dark Mac
/// the bar crew painted light carried the system's white title.
#[test]
fn the_painted_bar_carries_the_page_s_own_ink() {
    assert_eq!(
        wanted(0.8, (250, 250, 250), false, None),
        Some(([250, 250, 250], 1.0, false))
    );
    assert_eq!(
        wanted(0.8, (10, 10, 10), true, None),
        Some(([10, 10, 10], 1.0, true))
    );
}

/// A tube's bar frosts with its window instead of drawing a black strip
/// across the top of the glass.
#[test]
fn a_tube_s_title_bar_is_as_sheer_as_its_page() {
    assert_eq!(
        wanted(0.12, (2, 6, 5), true, Some(0.12)),
        Some(([2, 6, 5], 0.12, true))
    );
}

/// Glass's bar wears the frost it is handed, not its gaps': it carries the
/// window's title, and at the gaps' alpha a dark desktop swallowed it.
#[test]
fn a_glass_title_bar_is_as_frosted_as_its_panes() {
    assert_eq!(
        wanted(0.25, (250, 250, 250), false, Some(0.55)),
        Some(([250, 250, 250], 0.55, false))
    );
}

/// An Ink Reader page is opaque and still wears its own bar: on a dark Mac
/// the native one was a dark strip across the white page.
#[test]
fn an_ink_page_paints_its_bar_the_page_whatever_the_mac_is() {
    for id in [
        crew_theme::ThemeId::PaperLight,
        crew_theme::ThemeId::PaperDark,
    ] {
        let t = id.theme();
        let (r, g, b) = t.page_bg;
        assert_eq!(wear_for(t, 1.0), Some(([r, g, b], 1.0, t.dark)), "{id:?}");
    }
    let glass = crew_theme::ThemeId::GlassClear.theme();
    let l = glass.liquid.expect("glass");
    let o = l.window;
    let want = wanted(o, glass.page_bg, glass.dark, Some(l.cell_cover().max(o)));
    assert_eq!(wear_for(glass, o), want, "glass wears two sheets");
}

/// The user, 2026-10-17: "make title bar less transparent". Every glass
/// theme's bar hides more of the desktop than its panes, and most of it.
#[test]
fn a_glass_title_bar_is_thicker_than_its_panes() {
    for id in crew_theme::ALL_THEMES {
        let t = id.theme();
        let Some(l) = t.liquid else { continue };
        let Some((_, a, _)) = wear_for(t, l.window) else {
            panic!("{id:?}: a sheer window handed its bar to the OS");
        };
        assert!(a > l.pane_cover() && a >= 0.8, "{id:?}: bar alpha {a}");
    }
}

//! The hover card, rendered: a deep Far folder's row rested on, its full path beside the nav.
use super::*;

/// The card over a Far pane, in a dark and a light theme. The window's own frames, 350 ms
/// apart, carry the wait: the third is past [`DELAY_MS`].
/// `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew navtip_shot -- --ignored`
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn navtip_shot() {
    let _g = crate::app::theme_test_guard();
    crate::farrecent::clear();
    let base = std::env::temp_dir().join(format!("navtip-shot-{}", std::process::id()));
    let dir = base.join("clients/northwind/2026/quarterly-reports/drafts");
    std::fs::create_dir_all(&dir).unwrap();
    crate::farrecent::note(&dir);
    for id in [
        crew_theme::ThemeId::TokyoNight,
        crew_theme::ThemeId::Porcelain,
    ] {
        let name = format!("navtip-{id:?}").to_lowercase();
        let prep = |app: &mut CrewApp| {
            app.config.show_nav = true;
            app.config.nav_collapsed = false;
            app.config.nav_section = "far".into();
            app.input.focused = false;
            let (cw, ch, ..) = app.frame_geometry().unwrap();
            let (panel, _, _) = app.nav_hit_geometry().unwrap();
            // The pointer on the folder's row (content row 1: the card's border is row 0).
            app.cursor = (panel.x + 5.5 * cw, panel.y + 2.5 * ch);
            app.cursor_in = true;
        };
        let opacity = crate::tubesheer::sheer(id.theme());
        crate::crtglassshot_tests::window_with(&name, id, opacity, &["/far"], &prep);
    }
    crate::farrecent::clear();
    let _ = std::fs::remove_dir_all(&base);
}

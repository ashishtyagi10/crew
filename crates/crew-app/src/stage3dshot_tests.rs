//! The 3D backdrop (`/look 3d`) in a whole window, offscreen, at the
//! theme's own window opacity: the scene behind a working grid, seen through
//! the panes' smoked glass as the desktop is, on a dark, a light and a neon
//! glass page (a paper page is solid: nothing behind its panes shows) — each shot beside the same window with the backdrop off.
//!
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew stage3d_shot -- --ignored`
use crew_render::stage3d::{self, Stage};
use crew_theme::ThemeId;

fn shot(name: &str, id: ThemeId, on: bool) -> Option<Vec<u8>> {
    stage3d::set_stage(on.then_some(Stage::Orbit));
    stage3d::set_phase(0.1);
    crew_theme::set_theme(id);
    let opacity = crate::tubesheer::sheer(crew_theme::theme());
    let px = crate::crtglassshot_tests::window_with(name, id, opacity, &["/far", "/dash"], &|_| {});
    stage3d::set_stage(None);
    px
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn stage3d_shot_orbit_behind_a_grid() {
    let _g = crate::app::theme_test_guard();
    for id in [ThemeId::GlassNight, ThemeId::Blossom, ThemeId::NeonDark] {
        let n = id.as_str();
        let (Some(on), Some(off)) = (
            shot(&format!("stage3d-{n}"), id, true),
            shot(&format!("stage3d-{n}-off"), id, false),
        ) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        let changed = on
            .chunks(4)
            .zip(off.chunks(4))
            .filter(|(a, b)| a != b)
            .count();
        assert!(
            changed > 5_000,
            "{n}: the scene shows through: {changed} px"
        );
    }
}

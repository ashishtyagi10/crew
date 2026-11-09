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

/// The prism theme in a whole window: its scene comes from the theme
/// (`backdrop_3d` left at its default), published by the app as a frame
/// would, beside the same window with `/look 3d off`.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn stage3d_shot_prism_theme() {
    let _g = crate::app::theme_test_guard();
    let id = ThemeId::Prism;
    let opacity = crate::tubesheer::sheer(id.theme());
    let with = |name: &str, setting: &'static str| {
        let px =
            crate::crtglassshot_tests::window_with(name, id, opacity, &["/far", "/dash"], &|app| {
                app.config.backdrop_3d = setting.to_string();
                app.publish_stage_3d(false);
            });
        stage3d::set_stage(None);
        px
    };
    let (Some(on), Some(off)) = (
        with("stage3d-prism-theme", ""),
        with("stage3d-prism-theme-off", "off"),
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
        "prism's own scene shows through: {changed} px"
    );
}

/// The orbit on a tall window and an ultrawide one, each beside the same
/// window with the backdrop off: framed by the window's aspect, the knot is
/// on screen on the tall one (at a fixed x it was past the right edge) and
/// right of the text on the wide one (it was mid-window).
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn stage3d_shot_orbit_tall_and_wide() {
    let _g = crate::app::theme_test_guard();
    let id = ThemeId::GlassNight;
    let opacity = crate::tubesheer::sheer(id.theme());
    for (n, size) in [("tall", (480, 800)), ("wide", (1680, 560))] {
        let at = |on: bool| {
            stage3d::set_stage(on.then_some(Stage::Orbit));
            stage3d::set_phase(0.1);
            let name = format!("stage3d-orbit-{n}{}", if on { "" } else { "-off" });
            let px = crate::crtglassshot_tests::window_sized(
                &name,
                id,
                opacity,
                &["/far", "/dash"],
                &|_| {},
                size,
            );
            stage3d::set_stage(None);
            px
        };
        let (Some(on), Some(off)) = (at(true), at(false)) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        let changed = on
            .chunks(4)
            .zip(off.chunks(4))
            .filter(|(a, b)| a != b)
            .count();
        assert!(changed > 5_000, "{n}: the orbit shows: {changed} px");
    }
}

/// The orbit under a user's own accent, beside the same window at the theme's
/// accent: the knot's key light is the accent the app draws with, so a red
/// accent relights the scene (it took the theme's `accent_default` and the
/// two shots were the same picture).
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn stage3d_shot_orbit_custom_accent() {
    let _g = crate::app::theme_test_guard();
    let id = ThemeId::GlassNight;
    let opacity = crate::tubesheer::sheer(id.theme());
    let at = |name: &str, accent: Option<(u8, u8, u8)>| {
        stage3d::set_phase(0.1);
        let px =
            crate::crtglassshot_tests::window_with(name, id, opacity, &["/far", "/dash"], &|app| {
                if let Some(rgb) = accent {
                    crate::palette::set_accent(rgb);
                }
                app.config.backdrop_3d = "orbit".to_string();
                app.publish_stage_3d(false);
            });
        stage3d::set_stage(None);
        px
    };
    let red = at("stage3d-orbit-red-accent", Some((230, 40, 60)));
    let own = at("stage3d-orbit-theme-accent", None);
    crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
    let (Some(red), Some(own)) = (red, own) else {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
        return;
    };
    // Only the knot's box, low in the empty dash pane: the chrome wears the
    // accent anyway, and the dash's live numbers move between shots.
    let changed = (330..580u32)
        .flat_map(|y| (830..1100u32).map(move |x| ((y * 1280 + x) * 4) as usize))
        .filter(|&i| red[i..i + 3] != own[i..i + 3])
        .count();
    assert!(
        changed > 5_000,
        "the accent relights the knot: {changed} px"
    );
}

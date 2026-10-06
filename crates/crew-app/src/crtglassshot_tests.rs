//! The CRT family as the app draws it: a whole window — nav, panes, input
//! bar — through the real tube (bloom, scanlines, composite) over the page's
//! own backdrop, wash and lattice at rest. The glass goal
//! (`2026-10-05-crt-glass-tube-2300.md`) is judged on these: a tube is a
//! terminal running in glass, and that is a composition, not one card.
//!
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew crt_glass_shot -- --ignored`
use crate::app::CrewApp;
use crew_theme::ThemeId;

const W: u32 = 1280;
const H: u32 = 720;
const FONT_PX: f32 = 13.0;

/// The theme's backdrop at rest, as `frame.rs` builds it — on a row of
/// about `FONT_PX * 1.4` px (the lattice pitch clamps, so close is exact).
fn backdrop() -> Option<crew_render::ModernPaper> {
    let c = |rgb| {
        let [r, g, b, _] = crew_render::color::target_rgba(rgb, 1.0, false);
        [r, g, b]
    };
    crew_theme::theme().modern.map(|m| {
        let (spacing, radius) = crew_render::ModernPaper::cell_geometry(FONT_PX * 1.4);
        crew_render::ModernPaper {
            color_a: c(m.pole_a),
            color_b: c(m.pole_b),
            dots: m.dots,
            spacing,
            radius,
            wash: m.wash,
            clocks: Default::default(),
            focus: [0.5, 0.5],
            focus_pull: 0.0,
        }
    })
}

/// Shoot the window on `id` through the tube.
fn crt_window(name: &str, id: ThemeId) -> Option<Vec<u8>> {
    crew_theme::set_theme(id);
    // An unset accent follows the theme in the app (`accent_rgb`); the shot
    // never applies a config, so say so here or every tube wears the mint.
    crate::palette::set_accent(crew_theme::theme().accent_default);
    let px = crate::shotdraw_tests::draw_with(W, H, FONT_PX, true, backdrop(), |cw, ch| {
        let mut app = CrewApp {
            geo_override: Some((cw, ch, W as f32, H as f32, 1.0)),
            ..Default::default()
        };
        for cmd in ["/far", "/far", "/dash"] {
            app.submit_input(cmd.to_string());
        }
        app.zoomed = false;
        app.focused = 1;
        app.input.focused = false;
        for p in &mut app.panes {
            p.born_ms = 0;
        }
        app.build_frame();
        std::thread::sleep(std::time::Duration::from_millis(350));
        app.build_frame();
        std::thread::sleep(std::time::Duration::from_millis(350));
        app.build_frame()
    })?;
    crate::shotdraw_tests::write_png(name, &px, W, H);
    Some(px)
}

/// Every phosphor, as a window.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn crt_glass_shot() {
    let _g = crate::app::theme_test_guard();
    for id in [
        ThemeId::CrtGreen,
        ThemeId::CrtAmber,
        ThemeId::CrtBlue,
        ThemeId::CrtViolet,
    ] {
        let Some(px) = crt_window(&format!("crtglass-{}", id.as_str()), id) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(
            crate::shotgpu_tests::ink(&px) > 10_000,
            "{}: drew",
            id.as_str()
        );
    }
}

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
fn backdrop(clocks: crew_render::WashClocks) -> Option<crew_render::ModernPaper> {
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
            clocks,
            focus: [0.5, 0.5],
            focus_pull: 0.0,
        }
    })
}

/// Shoot the window on `id` through the tube, its page at `clocks`.
fn crt_window(name: &str, id: ThemeId, clocks: crew_render::WashClocks) -> Option<Vec<u8>> {
    window(name, id, clocks, crate::tubesheer::TUBE_OPACITY)
}

/// Shoot the window on `id`, its page at `clocks` and the window at `opacity`.
fn window(
    name: &str,
    id: ThemeId,
    clocks: crew_render::WashClocks,
    opacity: f32,
) -> Option<Vec<u8>> {
    window_with(
        name,
        id,
        clocks,
        opacity,
        &["/far", "/far", "/dash"],
        &|_| {},
    )
}

/// [`window`] with the panes `cmds` open, the second one focused, then
/// `prep` run on the app (a toast, text in the bar) before the frames.
pub(crate) fn window_with(
    name: &str,
    id: ThemeId,
    clocks: crew_render::WashClocks,
    opacity: f32,
    cmds: &[&str],
    prep: &dyn Fn(&mut CrewApp),
) -> Option<Vec<u8>> {
    crew_theme::set_theme(id);
    // The app serves a sheer window's palette with its frames and quiet
    // legends lifted (`glassborder`); so does the shot, or it shows inks the
    // window never draws.
    crew_theme::glassborder::set_sheer(opacity < 1.0);
    // An unset accent follows the theme in the app (`accent_rgb`); the shot
    // never applies a config, so say so here or every tube wears the mint.
    crate::palette::set_accent(crew_theme::theme().accent_default);
    let px = crate::shotdraw_tests::draw_with(
        W,
        H,
        FONT_PX,
        true,
        backdrop(clocks),
        opacity,
        |cw, ch| {
            let mut app = CrewApp {
                geo_override: Some((cw, ch, W as f32, H as f32, 1.0)),
                ..Default::default()
            };
            for cmd in cmds {
                app.submit_input(cmd.to_string());
            }
            app.zoomed = false;
            app.focused = 1;
            app.input.focused = false;
            for p in &mut app.panes {
                p.born_ms = 0;
            }
            prep(&mut app);
            app.build_frame();
            std::thread::sleep(std::time::Duration::from_millis(350));
            app.build_frame();
            std::thread::sleep(std::time::Duration::from_millis(350));
            app.build_frame()
        },
    );
    crew_theme::glassborder::set_sheer(false);
    let px = px?;
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
        let Some(px) = crt_window(&format!("crtglass-{}", id.as_str()), id, Default::default())
        else {
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

/// Every phosphor with its page awake — the glow beating under the glass —
/// so the page's depth can be judged where it actually moves.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn crt_glass_shot_awake() {
    let _g = crate::app::theme_test_guard();
    let clocks = crew_render::WashClocks {
        phase: 0.2,
        wander: 0.3,
        live: 1.0,
        eddy: 0.4,
    };
    for id in [
        ThemeId::CrtGreen,
        ThemeId::CrtAmber,
        ThemeId::CrtBlue,
        ThemeId::CrtViolet,
    ] {
        if crt_window(&format!("crtglass-awake-{}", id.as_str()), id, clocks).is_none() {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        }
    }
}

/// Liquid glass as a window: the panes refracting the wallpaper, at rest and
/// awake (the pools moved on, the glow beating), as sheer as the app ships it
/// — the alpha is the desktop showing through (`tubesheer::sheer`).
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn glass_window_shot() {
    let _g = crate::app::theme_test_guard();
    let awake = crew_render::WashClocks {
        phase: 0.2,
        wander: 0.3,
        live: 1.0,
        eddy: 0.4,
    };
    for id in [ThemeId::GlassSky, ThemeId::GlassDawn, ThemeId::GlassNight] {
        for (name, clocks) in [
            (format!("{}-window", id.as_str()), Default::default()),
            (format!("{}-window-awake", id.as_str()), awake),
        ] {
            let Some(px) = window(&name, id, clocks, crate::tubesheer::sheer(1.0, id.theme()))
            else {
                eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
                return;
            };
            assert!(crate::shotgpu_tests::ink(&px) > 10_000, "{name}: drew");
        }
    }
}

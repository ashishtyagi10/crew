//! The CRT family as the app draws it: a whole window — nav, panes, input
//! bar — through the real tube (bloom, scanlines, composite), as sheer as
//! the app ships it. The glass goal
//! (`2026-10-05-crt-glass-tube-2300.md`) is judged on these: a tube is a
//! terminal running in glass, and that is a composition, not one card.
//!
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew crt_glass_shot -- --ignored`
use crate::app::CrewApp;
use crew_theme::ThemeId;

const W: u32 = 1280;
const H: u32 = 720;
const FONT_PX: f32 = 13.0;

/// Shoot the window on `id`, as sheer as the app ships it.
fn window(name: &str, id: ThemeId) -> Option<Vec<u8>> {
    let opacity = crate::tubesheer::sheer(1.0, id.theme());
    window_with(name, id, opacity, &["/far", "/far", "/dash"], &|_| {})
}

/// [`window`] with the panes `cmds` open, the second one focused, then
/// `prep` run on the app (a toast, text in the bar) before the frames.
pub(crate) fn window_with(
    name: &str,
    id: ThemeId,
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
    let px = crate::shotdraw_tests::draw_with(W, H, FONT_PX, true, opacity, |cw, ch| {
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
    });
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
    for id in [ThemeId::CrtGreen, ThemeId::CrtAmber] {
        let Some(px) = window(&format!("crtglass-{}", id.as_str()), id) else {
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

/// Liquid glass as a window, as sheer as the app ships it — the alpha is
/// the desktop showing through (`tubesheer::sheer`).
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn glass_window_shot() {
    let _g = crate::app::theme_test_guard();
    for id in [ThemeId::GlassClear, ThemeId::GlassNight] {
        let name = format!("{}-window", id.as_str());
        let Some(px) = window(&name, id) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(crate::shotgpu_tests::ink(&px) > 10_000, "{name}: drew");
    }
}

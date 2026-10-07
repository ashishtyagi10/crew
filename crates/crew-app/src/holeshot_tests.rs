//! The black hole as the app draws it: a whole window — nav, panes, input
//! bar — over an awake page, so the hole can be judged where it actually
//! lives: behind the work, seen between the cards and through their glass.
//!
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew hole_shot -- --ignored`
use crate::app::CrewApp;
use crew_theme::ThemeId;

const W: u32 = 1280;
const H: u32 = 720;
const FONT_PX: f32 = 13.0;

/// The theme's backdrop at `clocks`, as `frame.rs` builds it.
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

/// Shoot the window on `id`, its page at `clocks`, with `cmds` run first.
fn window(
    name: &str,
    id: ThemeId,
    clocks: crew_render::WashClocks,
    cmds: &[&str],
) -> Option<Vec<u8>> {
    crew_theme::set_theme(id);
    crate::palette::set_accent(crew_theme::theme().accent_default);
    let px =
        crate::shotdraw_tests::draw_with(W, H, FONT_PX, false, backdrop(clocks), 1.0, |cw, ch| {
            let mut app = CrewApp {
                geo_override: Some((cw, ch, W as f32, H as f32, 1.0)),
                ..Default::default()
            };
            for cmd in cmds {
                app.submit_input(cmd.to_string());
            }
            app.zoomed = false;
            app.focused = 0;
            app.input.focused = false;
            for p in &mut app.panes {
                p.born_ms = 0;
            }
            app.build_frame();
            std::thread::sleep(std::time::Duration::from_millis(350));
            app.build_frame()
        })?;
    crate::shotdraw_tests::write_png(name, &px, W, H);
    Some(px)
}

/// A dark and a light theme with the page awake, over one pane and over two.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn hole_shot() {
    let _g = crate::app::theme_test_guard();
    let clocks = crew_render::WashClocks {
        phase: 0.2,
        wander: 0.3,
        live: 1.0,
        eddy: 0.4,
        ping: -1.0,
    };
    for id in [ThemeId::Nebula, ThemeId::Blossom] {
        for (tag, cmds) in [("one", &["/far"][..]), ("two", &["/far", "/dash"][..])] {
            let name = format!("hole-{}-{tag}", id.as_str());
            if window(&name, id, clocks, cmds).is_none() {
                eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
                return;
            }
        }
    }
}

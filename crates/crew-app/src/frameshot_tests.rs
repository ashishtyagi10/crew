//! The whole window, offscreen: a real `CrewApp` building a real frame —
//! nav, panes, input bar, all of it — through the same cell grid the app
//! draws with.
//!
//! Every other shot frames one surface. The glass look is a COMPOSITION —
//! how the cards sit on the page and against each other, which one has
//! lifted, where their shadows fall — and none of that is visible one card
//! at a time. `frame_geometry` answers from `geo_override`, so no window or
//! surface is needed.
//!
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew frame_shot -- --ignored`
use crate::app::CrewApp;
use crew_theme::ThemeId;

const W: u32 = 1280;
const H: u32 = 720;

/// Shoot the window on theme `id` after `setup` has arranged it. Built three
/// times across ~0.6 s so assembly, glide and the focus lift have settled —
/// a first frame is a picture of cards still drawing themselves in.
pub(crate) fn frame_shot(
    name: &str,
    id: ThemeId,
    setup: impl FnOnce(&mut CrewApp),
) -> Option<Vec<u8>> {
    crew_theme::set_theme(id);
    let px = crate::shotdraw_tests::draw(W, H, 13.0, |cw, ch| {
        let mut app = CrewApp {
            geo_override: Some((cw, ch, W as f32, H as f32, 1.0)),
            ..Default::default()
        };
        setup(&mut app);
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

/// Two file managers and a dashboard, the second far focused — the grid a
/// working session looks like, on the light and dark page of each family.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn frame_shot_every_family() {
    let _g = crate::app::theme_test_guard();
    for id in [
        ThemeId::PaperLight,
        ThemeId::PaperDark,
        ThemeId::Blossom,
        ThemeId::Nebula,
        ThemeId::CrtGreen,
    ] {
        let shot = frame_shot(&format!("frame-{}", id.as_str()), id, |app| {
            for cmd in ["/far", "/far", "/dash"] {
                app.submit_input(cmd.to_string());
            }
            // `/dash` opens zoomed; the point here is the grid.
            app.zoomed = false;
            app.focused = 1;
            app.input.focused = false;
        });
        let Some(px) = shot else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(
            crate::shotgpu_tests::ink(&px) > 10_000,
            "{}: the frame drew",
            id.as_str()
        );
    }
}

/// The pointer resting on an unfocused card: it rises a little on its glass
/// (a softer, lower shadow than the focused card's), set beside
/// `frame-paper-light.png`.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn frame_shot_hover_lift() {
    let _g = crate::app::theme_test_guard();
    let shot = frame_shot("frame-paper-light-hover", ThemeId::PaperLight, |app| {
        for cmd in ["/far", "/far", "/dash"] {
            app.submit_input(cmd.to_string());
        }
        app.zoomed = false;
        app.focused = 1;
        app.input.focused = false;
        app.cursor = (300.0, 150.0);
        app.cursor_in = true;
    });
    if shot.is_none() {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
    }
}

/// The first thing a session shows: the welcome pane, the glance nav and
/// the input bar with its cwd legend, on a dark and a light page.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn frame_shot_welcome() {
    let _g = crate::app::theme_test_guard();
    for id in [ThemeId::PaperDark, ThemeId::Nebula, ThemeId::PaperLight] {
        let shot = frame_shot(&format!("frame-welcome-{}", id.as_str()), id, |app| {
            app.input.cwd = "~/code/crew".into();
            app.input.focused = true;
        });
        if shot.is_none() {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        }
    }
}

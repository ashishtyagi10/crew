use crate::app::CrewApp;
use crew_render::PaneScene;
use crew_theme::ThemeId;

/// An app with two panes on the grid and the pointer resting on the first.
fn app_pointing_at_a_pane() -> CrewApp {
    let mut app = CrewApp {
        geo_override: Some((8.0, 17.0, 1280.0, 720.0, 1.0)),
        ..Default::default()
    };
    app.submit_input("/far".to_string());
    app.submit_input("/far".to_string());
    app.input.focused = false;
    for p in &mut app.panes {
        p.born_ms = 0;
        p.zoom = Some(false); // both cards on the grid, as the shots set them
    }
    app.build_frame();
    let r = app.panes[0].rect;
    app.cursor = (r.x + r.w / 2.0, r.y + r.h / 2.0);
    app.cursor_in = true;
    app
}

/// The card under the pointer rises (that is `hoverlift`'s job) — but not
/// while the keys panel is up: the panes take nothing then, so nothing under
/// the panel answers the pointer. Motion off makes the glide land in one
/// frame, so one frame is the whole answer.
#[test]
fn nothing_rises_under_the_keys_panel() {
    let _g = crate::app::motion_test_guard();
    let was = crate::motion::level();
    crate::motion::set_level(crate::motion::MotionLevel::Off);
    let mut app = app_pointing_at_a_pane();
    assert_eq!(app.pane_at_cursor(), Some(0), "the pointer is on pane 0");
    app.build_frame();
    let lit = app.hover_lift.lift(0);
    app.help_open = true;
    app.hover_moved();
    app.build_frame();
    let under = app.hover_lift.lift(0);
    crate::motion::set_level(was);
    assert_eq!(lit, crate::hoverlift::HOVER, "with the panel shut it rises");
    assert_eq!(under, 0.0, "under the panel it settles to the page");
}

/// With the panel open, every cell of the panes under it is washed by
/// exactly [`super::DIM`] — on an opaque page, on see-through glass (where
/// the wash stops at the UI floor) and on prism — and a good share of their
/// ink visibly moves; the panel itself keeps full ink.
#[test]
fn the_keys_panel_sets_every_pane_under_it_back() {
    let _g = crate::app::theme_test_guard();
    for id in [
        ThemeId::PaperDark,
        ThemeId::PaperLight,
        ThemeId::GlassNight,
        ThemeId::GlassClear,
        ThemeId::Prism,
    ] {
        crew_theme::set_theme(id);
        let mut app = app_pointing_at_a_pane();
        let open = app.build_frame();
        app.help_open = true;
        let shut = app.build_frame();
        let pane = app.panes[0].rect;
        let at = |v: &[PaneScene]| {
            v.iter()
                .filter(|s| !s.overlay && s.x > pane.x && s.y > pane.y)
                .find(|s| s.x < pane.x + pane.w && s.y < pane.y + pane.h)
                .map(|s| s.cells.clone())
                .expect("pane 0's content scene")
        };
        let (mut want, got) = (at(&open), at(&shut));
        let lit: Vec<_> = want.iter().map(|c| c.fg).collect();
        crate::spotlight::wash(&mut want, super::DIM);
        // Two frames apart, so allow the breath of a frame's animated ink.
        let near = |a: (u8, u8, u8), b: (u8, u8, u8)| {
            let d = |x: u8, y: u8| x.abs_diff(y) <= 2;
            d(a.0, b.0) && d(a.1, b.1) && d(a.2, b.2)
        };
        assert_eq!(got.len(), want.len(), "{}: same cells", id.as_str());
        for (g, w) in got.iter().zip(&want) {
            assert!(
                near(g.fg, w.fg),
                "{}: {:?} washed by DIM is {:?}, drawn {:?}",
                id.as_str(),
                (g.col, g.row, g.c),
                w.fg,
                g.fg
            );
        }
        let inked = got.iter().filter(|c| c.c != ' ').count();
        let moved = got
            .iter()
            .zip(&lit)
            .filter(|(c, was)| c.c != ' ' && c.fg != **was)
            .count();
        assert!(
            moved * 4 > inked,
            "{}: {moved} of {inked} cells dimmed",
            id.as_str()
        );
        let panel = shut
            .iter()
            .filter(|s| s.overlay && s.glass)
            .max_by(|a, b| (a.w * a.h).total_cmp(&(b.w * b.h)))
            .expect("the keys panel");
        let (hw, hh) = ((panel.w / 8.0) as u16, (panel.h / 17.0) as u16);
        let here = crate::helphere::section_for(&app.panes[app.focused].content);
        let drawn = crate::help::help_cells(hw, hh, app.help_scroll, &app.help_filter, here);
        let fg = |c: &[crew_render::CellView]| c.iter().map(|c| c.fg).collect::<Vec<_>>();
        assert_eq!(
            fg(&panel.cells),
            fg(&drawn),
            "{}: the panel keeps full ink",
            id.as_str()
        );
    }
}

/// The overlay as the app draws it, pointer on a pane, on an opaque page,
/// see-through glass and prism. Look at these.
///
/// `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew keys_scrim_shot -- --ignored`
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn keys_scrim_shot() {
    let _g = crate::app::theme_test_guard();
    for id in [ThemeId::PaperDark, ThemeId::GlassNight, ThemeId::Prism] {
        let opacity = crate::tubesheer::sheer(id.theme());
        let name = format!("keys-scrim-{}", id.as_str());
        let prep = |app: &mut CrewApp| {
            app.build_frame();
            let r = app.panes[1].rect;
            app.cursor = (r.x + 20.0, r.y + 20.0);
            app.cursor_in = true;
            app.help_open = true;
        };
        let cmds = ["/far", "/far", "/dash"];
        if crate::crtglassshot_tests::window_with(&name, id, opacity, &cmds, &prep).is_none() {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        }
    }
}

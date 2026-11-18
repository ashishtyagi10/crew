//! One shot per palette of a thing every palette draws its own way: what
//! the theme loop of 2026-11-14 looked at, round by round. The glass survey
//! shoots five windows a palette and takes six minutes; these take one.
//!
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew theme_welcome_shot -- --ignored`
use crew_theme::ALL_THEMES;

/// The welcome on every palette: its rain falls in the palette's own hue
/// ([`crate::rainink`]).
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn theme_welcome_shot() {
    let _g = crate::app::theme_test_guard();
    for id in ALL_THEMES {
        let name = format!("welcome-{}", id.as_str());
        let sheer = crate::tubesheer::sheer(id.theme());
        if crate::crtglassshot_tests::window_with(&name, id, sheer, &[], &|_| {}).is_none() {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        }
    }
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
}

/// The raised palettes' (soft's and metal's) in-card boxes, pressed in as wells
/// ([`crate::reliefwell`]): the settings form beside the to-do list.
///
/// `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew theme_wells_shot -- --ignored`
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn theme_wells_shot() {
    let _g = crate::app::theme_test_guard();
    let soft = |id: &crew_theme::ThemeId| id.theme().relief() > 0.0;
    for id in ALL_THEMES.into_iter().filter(soft) {
        let name = format!("wells-{}", id.as_str());
        let sheer = crate::tubesheer::sheer(id.theme());
        let cmds = ["/todo", "/settings"];
        if crate::crtglassshot_tests::window_with(&name, id, sheer, &cmds, &|_| {}).is_none() {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        }
    }
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
}

/// The open nav on every palette, in tabs, beside the dashboard, the
/// settings form and the to-do list: what the second theme loop of
/// 2026-11-14 looked at.
///
/// `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew theme_nav_shot -- --ignored`
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn theme_nav_shot() {
    let _g = crate::app::theme_test_guard();
    for id in ALL_THEMES {
        let name = format!("nav-{}", id.as_str());
        let sheer = crate::tubesheer::sheer(id.theme());
        let prep = |app: &mut crate::app::CrewApp| {
            app.config.show_nav = true;
            app.config.nav_collapsed = false;
            crate::tabshot_tests::three_tabs(app);
        };
        let cmds = ["/dash", "/settings", "/todo", "/far"];
        if crate::crtglassshot_tests::window_with(&name, id, sheer, &cmds, &prep).is_none() {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        }
    }
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
}

/// `/far` on the raised palettes (soft's and metal's): its panels are wells, and the cursor bar
/// rides inside one.
///
/// `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew theme_far_shot -- --ignored`
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn theme_far_shot() {
    let _g = crate::app::theme_test_guard();
    let soft = |id: &crew_theme::ThemeId| id.theme().relief() > 0.0;
    for id in ALL_THEMES.into_iter().filter(soft) {
        let name = format!("far-{}", id.as_str());
        let sheer = crate::tubesheer::sheer(id.theme());
        if crate::crtglassshot_tests::window_with(&name, id, sheer, &["/far"], &|_| {}).is_none() {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        }
    }
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
}

/// The metal palettes' brushed plates (the user, 2026-11-17: "actual metal
/// look … some shiny look"): the open nav beside the dashboard and the to-do
/// list, once with the pointer outside the window (the sheen at rest) and
/// once at its right edge, so the sheen is seen to slide.
///
/// `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew theme_metal_shot -- --ignored`
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn theme_metal_shot() {
    let _g = crate::app::theme_test_guard();
    let metal = |id: &crew_theme::ThemeId| id.mode() == crew_theme::RandomMode::Metal;
    for id in ALL_THEMES.into_iter().filter(metal) {
        let sheer = crate::tubesheer::sheer(id.theme());
        for lean in [false, true] {
            let name = format!("metal-{}{}", id.as_str(), if lean { "-lean" } else { "" });
            let prep = |app: &mut crate::app::CrewApp| {
                app.config.show_nav = true;
                app.config.nav_collapsed = false;
            };
            let tilt = if lean { (0.95, 0.0) } else { (0.0, 0.0) };
            crate::shotdraw_tests::TILT.with(|t| t.set(tilt));
            let cmds = ["/dash", "/todo"];
            let px = crate::crtglassshot_tests::window_with(&name, id, sheer, &cmds, &prep);
            crate::shotdraw_tests::TILT.with(|t| t.set((0.0, 0.0)));
            if px.is_none() {
                eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
                return;
            }
        }
    }
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
}

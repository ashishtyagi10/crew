//! The neumorphic themes as the app draws them: a whole window (nav, panes,
//! input bar) on each relief palette, as sheer as the app ships it. The look
//! is a composition: unframed cards told from their page by two shadows, and
//! an input bar pressed into it.
//!
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew relief_window_shot -- --ignored`
use crew_theme::{RandomMode, ThemeId, ALL_THEMES};

/// A relief card is the page's own colour, so with its frame gone a window
/// of them is mostly that one colour: the page outnumbers every other
/// colour in the shot many times over.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn relief_window_shot() {
    let _g = crate::app::theme_test_guard();
    for id in ALL_THEMES
        .into_iter()
        .filter(|id| id.mode() == RandomMode::Relief)
    {
        let name = format!("{}-window", id.as_str());
        let opacity = crate::tubesheer::sheer(id.theme());
        let cmds = ["/far", "/far", "/dash"];
        let Some(px) = crate::crtglassshot_tests::window_with(&name, id, opacity, &cmds, &|_| {})
        else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(crate::shotgpu_tests::ink(&px) > 10_000, "{name}: drew");
    }
    crew_theme::set_theme(ThemeId::PaperDark);
}

/// What floats over the cards on a relief palette: the command palette open
/// from the bar, a toast and an alert. Look at these.
///
/// `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew relief_popup_shot -- --ignored`
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn relief_popup_shot() {
    let _g = crate::app::theme_test_guard();
    for id in [ThemeId::Porcelain, ThemeId::Slate] {
        let name = format!("{}-popup", id.as_str());
        let opacity = crate::tubesheer::sheer(id.theme());
        let prep = |app: &mut crate::app::CrewApp| {
            app.input.focused = true;
            app.input.text = "/th".into();
            let now = crate::anim::now_ms();
            app.toasts
                .push("swarm finished · 3 tasks".into(), "done", false, now);
            app.toasts
                .push("cargo test failed".into(), "bell", true, now);
        };
        let cmds = ["/far", "/dash"];
        let Some(px) = crate::crtglassshot_tests::window_with(&name, id, opacity, &cmds, &prep)
        else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(crate::shotgpu_tests::ink(&px) > 10_000, "{name}: drew");
    }
    crew_theme::set_theme(ThemeId::PaperDark);
}

/// The nav open beside the panes on a relief palette: its sections and its
/// tab cards moulded like every other card. Look at these.
///
/// `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew relief_nav_shot -- --ignored`
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn relief_nav_shot() {
    let _g = crate::app::theme_test_guard();
    for id in [ThemeId::Porcelain, ThemeId::Slate] {
        let name = format!("{}-nav", id.as_str());
        let opacity = crate::tubesheer::sheer(id.theme());
        let prep = |app: &mut crate::app::CrewApp| {
            app.config.show_nav = true;
            app.config.nav_collapsed = false;
            // Slate shows the nav in tabs: a tile per tab's card.
            if id == ThemeId::Slate {
                crate::tabshot_tests::three_tabs(app);
            }
        };
        let cmds = ["/far", "/dash", "/todo", "/far"];
        let Some(px) = crate::crtglassshot_tests::window_with(&name, id, opacity, &cmds, &prep)
        else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(crate::shotgpu_tests::ink(&px) > 10_000, "{name}: drew");
    }
    crew_theme::set_theme(ThemeId::PaperDark);
}

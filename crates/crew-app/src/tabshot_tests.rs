//! Tabs as the app draws them: a whole window with three tabs, the nav's
//! PANES list a bordered card per tab, open and folded to the rail.
//!
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew tab_shot -- --ignored`
use crate::app::CrewApp;
use crate::crtglassshot_tests::window_with;
use crew_theme::ThemeId;

/// Panes 1-2 in tab 1 (`build`), 3 in tab 2, 4 in tab 3 (`web`); tab 1 on.
fn three_tabs(app: &mut CrewApp) {
    let (a, b, c) = (app.tabs.active, app.tabs.push(), app.tabs.push());
    for (i, t) in [a, a, b, c].into_iter().enumerate() {
        app.panes[i].tab = t;
    }
    app.tabs.get_mut(a).expect("tab").name = Some("build".into());
    app.tabs.get_mut(c).expect("tab").name = Some("web".into());
    crate::attention::raise(&mut app.panes[3], crate::notify::NotifyKind::AgentDone, 0);
    app.focused = 1;
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn tab_shot() {
    let cmds = ["/far", "/dash", "/todo", "/far"];
    let _g = crate::appearanceguard::theme_test_guard();
    for (name, folded) in [("tabs-nav", false), ("tabs-rail", true)] {
        window_with(name, ThemeId::PaperDark, 1.0, &cmds, &|app| {
            three_tabs(app);
            app.config.nav_collapsed = folded;
        });
    }
}

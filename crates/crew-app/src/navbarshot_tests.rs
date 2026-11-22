//! Off-screen shots of the activity bar: every section open, and the bar collapsed, on a dark,
//! a soft (relief) and a glass theme.
//!
//! `CREW_SHOT_DIR=<dir> [CREW_SHOT_FONT="JetBrainsMono Nerd Font Mono"] \
//!  cargo test -p crew-app --bin crew navbar_shot -- --ignored`
use crate::navsection::NavSection;
use crew_theme::ThemeId;

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn navbar_shot() {
    let _g = crate::app::theme_test_guard();
    let icons = match std::env::var("CREW_SHOT_FONT") {
        Ok(_) => "icons",
        Err(_) => "plain",
    };
    for id in [ThemeId::TokyoNight, ThemeId::Porcelain, ThemeId::GlassNight] {
        let opacity = crate::tubesheer::sheer(id.theme());
        let states = NavSection::ALL.map(Some).into_iter().chain([None]);
        for state in states {
            let label = state.map_or("collapsed", NavSection::as_str);
            let name = format!("navbar-{icons}-{id:?}-{label}").to_lowercase();
            let prep = move |app: &mut crate::app::CrewApp| {
                app.config.show_nav = true;
                app.config.nav_collapsed = state.is_none();
                if let Some(s) = state {
                    app.config.nav_section = s.as_str().to_string();
                }
                // The tree with a folder open: its guides beside the bar's edge.
                if state == Some(NavSection::Explorer) {
                    let root = app.explorer_root();
                    let items = app.sidebar.explorer.items(&root);
                    if let Some(f) = items.iter().find(|i| i.dir && i.depth == 1) {
                        let f = f.clone();
                        app.sidebar.explorer.toggle(&f);
                    }
                }
                app.log = (0..12)
                    .map(|i| crate::applog::LogEntry {
                        level: crate::applog::LogLevel::Info,
                        text: format!("12:{i:02} saved notes.md"),
                    })
                    .collect();
            };
            crate::crtglassshot_tests::window_with(&name, id, opacity, &["/far", "/far"], &prep);
        }
    }
}

/// Two tabs on a relief theme: the tab cards are wells in the panel, beside the bar's groove.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn navbar_shot_tabs() {
    let _g = crate::app::theme_test_guard();
    for id in [ThemeId::Porcelain, ThemeId::TokyoNight] {
        let name = format!("navbar-tabs-{id:?}").to_lowercase();
        let prep = |app: &mut crate::app::CrewApp| {
            app.config.show_nav = true;
            app.config.nav_collapsed = false;
            app.config.nav_section = "panes".into();
            app.reconcile_grid();
            let second = app.tabs.add();
            app.activate_tab(second);
            app.submit_input("/far".to_string());
            app.focused = app.panes.len() - 1;
            app.reconcile_grid();
        };
        let opacity = crate::tubesheer::sheer(id.theme());
        crate::crtglassshot_tests::window_with(&name, id, opacity, &["/far", "/far"], &prep);
    }
}

/// The tree holding the keys: a folder open, the cursor two rows down, the pane unringed.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn navbar_shot_tree_keys() {
    let _g = crate::app::theme_test_guard();
    for id in [ThemeId::TokyoNight, ThemeId::Porcelain] {
        let name = format!("navbar-treekeys-{id:?}").to_lowercase();
        let prep = |app: &mut crate::app::CrewApp| {
            app.explorer_keys_toggle();
            let down = winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowDown);
            app.explorer_tree_key(&down, true);
            app.explorer_tree_key(&down, true);
        };
        let opacity = crate::tubesheer::sheer(id.theme());
        crate::crtglassshot_tests::window_with(&name, id, opacity, &["/far"], &prep);
    }
}

/// The tree over crew's own repository with its git marks, once the nav's poll has answered.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn navbar_shot_tree_git() {
    let _g = crate::app::theme_test_guard();
    for id in [ThemeId::TokyoNight, ThemeId::Porcelain] {
        let name = format!("navbar-treegit-{id:?}").to_lowercase();
        let prep = |app: &mut crate::app::CrewApp| {
            app.explorer_keys_toggle();
            let root = app.explorer_root();
            let src = app
                .sidebar
                .explorer
                .items(&root)
                .into_iter()
                .find(|i| i.name == "src");
            if let Some(src) = src {
                app.sidebar.explorer.toggle(&src);
            }
            // The cursor on this round's own new file, so the changed rows are in view.
            app.sidebar.explorer.keys = None;
            app.sidebar.explorer.picked = Some(root.join("src/explorergit.rs"));
            app.explorer_keys_toggle();
            for t in 0..400 {
                app.git_fleet.poll(std::slice::from_ref(&root), t * 10);
                if app.git_fleet.files(&root).is_some() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        };
        let opacity = crate::tubesheer::sheer(id.theme());
        crate::crtglassshot_tests::window_with(&name, id, opacity, &["/far"], &prep);
    }
}

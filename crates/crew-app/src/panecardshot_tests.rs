//! Off-screen shots of the PANES list as cards: five agent panes and a
//! shell in one tab, Far in another — working, waiting on you, done, idle —
//! on a dark, a soft (relief) and a glass theme, and in a short window where
//! the cards give way to one-line rows.
//!
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew panecard_shot -- --ignored`
use crate::notify::NotifyKind;
use crate::pane::PaneContent;
use crew_theme::ThemeId;

fn rows(lines: &[&str]) -> Vec<String> {
    lines.iter().map(|s| s.to_string()).collect()
}

/// Turn the app's terminals into the crew in the user's screenshot.
fn crew(app: &mut crate::app::CrewApp) {
    let now = crate::anim::now_ms();
    let home = std::env::var("HOME").unwrap_or_default();
    let screens: [(&str, Option<&str>, Vec<String>); 5] = [
        (
            "jessi",
            Some("claude"),
            rows(&[
                "⏺ Reading src/gen.rs",
                "✳ Pondering… (12s · esc to interrupt)",
                "  ? for shortcuts",
            ]),
        ),
        (
            "crew",
            Some("claude"),
            rows(&[
                "│ Do you want to proceed?   │",
                "│ ❯ 1. Yes                  │",
            ]),
        ),
        ("crew", None, Vec::new()),
        (
            "terra",
            Some("claude"),
            rows(&["⏺ Updated 3 files; the tests pass.", "│ >            │"]),
        ),
        (
            "sentinel",
            Some("claude"),
            rows(&["⏺ Watching 3 panes for a question.", "│ >            │"]),
        ),
    ];
    for (k, (dir, cmd, screen)) in screens.into_iter().enumerate() {
        let Some(p) = app.panes.get_mut(k) else { break };
        p.dir = Some(std::path::PathBuf::from(format!("{home}/code/{dir}")));
        if let PaneContent::Terminal(t) = &mut p.content {
            t.cmd = cmd.map(str::to_string);
            // The working one moved just now; the others have long been still.
            match (cmd, k) {
                (Some(_), 0) => _ = t.tail.gist.note(&screen, now),
                (Some(_), _) => t.tail.gist = crate::panegist::Gist::settled(&screen),
                (None, _) => {}
            }
        }
    }
    crate::attention::raise(
        &mut app.panes[1],
        NotifyKind::Waiting,
        now.saturating_sub(10_000),
    );
    crate::attention::raise(
        &mut app.panes[3],
        NotifyKind::AgentDone,
        now.saturating_sub(10_000),
    );
}

fn prep(app: &mut crate::app::CrewApp) {
    app.config.show_nav = true;
    app.config.nav_collapsed = false;
    app.config.nav_section = "panes".into();
    for _ in 0..5 {
        app.spawn_new_pane();
    }
    crew(app);
    app.reconcile_grid();
    let first = app.tabs.active;
    let second = app.tabs.add();
    app.activate_tab(second);
    app.submit_input("/far".to_string());
    app.reconcile_grid();
    app.activate_tab(first);
    app.focused = 4;
    app.reconcile_grid();
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn panecard_shot() {
    let _g = crate::app::theme_test_guard();
    for id in [ThemeId::TokyoNight, ThemeId::Porcelain, ThemeId::GlassNight] {
        let name = format!("panecards-{id:?}").to_lowercase();
        let opacity = crate::tubesheer::sheer(id.theme());
        crate::crtglassshot_tests::window_with(&name, id, opacity, &[], &prep);
    }
    // Too short for every card's two lines: the one-line rows come back.
    let short = (1280, 300);
    let id = ThemeId::TokyoNight;
    crate::crtglassshot_tests::window_sized("panecards-short", id, 1.0, &[], &prep, short);
}

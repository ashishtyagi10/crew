//! The glass themes over more of the app than `glass_window_shot`'s two
//! file panels and a dashboard: every pane kind that opens without a shell
//! or a network, each window shot as sheer as the app ships it (the alpha is
//! the desktop showing through). Composite over a dark and a light desktop
//! to judge it — the glass's worst cases are those two.
//!
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew glass_survey_shot -- --ignored`
use crew_theme::ThemeId;

/// Pane sets, one window each: name and the commands that open its panes.
const SETS: &[(&str, &[&str])] = &[
    ("todo-keys", &["/todo", "/keys"]),
    ("usage-settings", &["/usage", "/settings"]),
    ("view-dash", &["/view Cargo.toml", "/dash"]),
];

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn glass_survey_shot() {
    let _g = crate::app::theme_test_guard();
    for id in [ThemeId::GlassSky, ThemeId::GlassNight] {
        let sheer = crate::tubesheer::sheer(1.0, id.theme());
        for (set, cmds) in SETS {
            let name = format!("survey-{}-{set}", id.as_str());
            let shot =
                crate::crtglassshot_tests::window_with(&name, id, Default::default(), sheer, cmds);
            if shot.is_none() {
                eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
                return;
            }
        }
    }
}

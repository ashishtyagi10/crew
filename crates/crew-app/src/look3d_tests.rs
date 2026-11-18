use super::*;
use crew_theme::ThemeId;

#[test]
fn a_saved_setting_keeps_only_a_scene_it_knows() {
    assert_eq!(clean("orbit".into()), "orbit");
    assert_eq!(clean(" prism ".into()), "prism");
    assert_eq!(clean("galaxy".into()), "");
    assert_eq!(clean(String::new()), "");
    // `off` is a choice now, not the default: it must survive a reload, or
    // turning prism's scene off would come undone at the next launch.
    assert_eq!(clean("off".into()), "off");
    assert_eq!(clean("none".into()), "off");
}

#[test]
fn the_ladder_offers_theme_off_then_every_scene() {
    let rows = ladder();
    assert_eq!(rows[0].0, "theme");
    assert_eq!(rows[1].0, "off");
    assert_eq!(rows.len(), 2 + Stage::ALL.len());
    assert!(rows.iter().any(|(n, _)| n == "orbit"));
    assert!(rows.iter().any(|(n, _)| n == "prism"));
}

/// The prism theme turns its scene on by itself; `off` turns it off; a
/// theme with no scene draws none by default; a named scene wins on any
/// theme.
#[test]
fn the_prism_theme_turns_its_scene_on_by_default() {
    assert_eq!(resolve("", ThemeId::Prism), Some(Stage::Prism));
    assert_eq!(resolve("off", ThemeId::Prism), None);
    assert_eq!(resolve("", ThemeId::GlassNight), None);
    assert_eq!(resolve("orbit", ThemeId::Prism), Some(Stage::Orbit));
    assert_eq!(resolve("prism", ThemeId::GlassNight), Some(Stage::Prism));
}

/// What a frame publishes follows the live theme: the default setting
/// draws prism's scene on prism and nothing once the theme moves on.
#[test]
fn the_frame_publishes_the_live_themes_scene() {
    let _g = crate::app::theme_test_guard();
    let mut app = CrewApp::default();
    app.config.backdrop_3d.clear();
    crew_theme::set_theme(ThemeId::Prism);
    app.publish_stage_3d(false);
    assert_eq!(stage3d::stage(), Some(Stage::Prism));
    crew_theme::set_theme(ThemeId::Nebula);
    app.publish_stage_3d(false);
    assert_eq!(stage3d::stage(), None);
    crew_theme::set_theme(ThemeId::PaperDark);
}

/// A frame hands the scene the accent the app draws with, so a user's own
/// `accent` lights the knot; before, it was the theme's `accent_default`.
#[test]
fn the_frame_lights_the_scene_in_the_users_accent() {
    let _g = crate::app::theme_test_guard();
    let mut app = CrewApp::default();
    app.config.backdrop_3d = "orbit".to_string();
    crate::palette::set_accent((230, 40, 60));
    app.publish_stage_3d(false);
    let lit = stage3d::key();
    let drawn = crate::palette::accent();
    crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
    stage3d::set_stage(None);
    assert_eq!(lit, Some(drawn));
    assert_ne!(drawn, crew_theme::theme().accent_default);
}

/// The settings form offers the 3D backdrop: its picker steps the same
/// settings `/look 3d` keeps (theme → off → orbit → prism → theme), shows the
/// default as `theme`, and Save carries the pick into the config.
#[test]
fn the_settings_form_steps_and_saves_the_backdrop() {
    use crate::settingspane::{cycle_value, Field, SettingsAction, SettingsPane, FIELDS};
    let mut p = SettingsPane::new(crate::config::CrewConfig::default(), vec![]);
    p.focus = FIELDS.iter().position(|&f| f == Field::Backdrop3d).unwrap();
    let mut seen = vec![p.draft.backdrop_3d.clone()];
    for _ in 0..4 {
        cycle_value(&mut p, false);
        seen.push(p.draft.backdrop_3d.clone());
    }
    assert_eq!(seen, ["", "off", "orbit", "prism", ""]);
    assert_eq!(label(""), "theme");
    cycle_value(&mut p, true);
    assert_eq!(p.draft.backdrop_3d, "prism", "← steps back and wraps");
    cycle_value(&mut p, true);
    cycle_value(&mut p, true);
    let SettingsAction::Apply(cfg) = p.save() else {
        panic!("save must apply");
    };
    assert_eq!(cfg.backdrop_3d, "off");
}

//! Tests for the config/theme plumbing in `spawn.rs`.
use crate::app::CrewApp;
use crate::config::CrewConfig;

#[test]
fn hydrated_env_hands_spawns_the_detection_path() {
    // Run panes must execute against the SAME PATH commands are detected
    // with (cmdcheck::effective_path) — a Dock-launched app's inherited PATH
    // misses ~/.local/bin and /opt/homebrew/bin, so `claude` would pass
    // detection yet fail to spawn.
    let env = crate::spawn::hydrated_env();
    let path = env
        .iter()
        .find(|(k, _)| k == "PATH")
        .map(|(_, v)| v.clone());
    assert_eq!(path, Some(crate::cmdcheck::effective_path()));
    assert!(!path.unwrap().is_empty());
}

#[test]
fn apply_config_adopts_values_without_a_renderer() {
    // `apply_config` pins a theme app-wide (spawn.rs: no `config.theme` →
    // `apply_selection(Fixed(theme_id()))`, which clears the random MODE), so
    // even a test that only cares about font_size/show_nav must take the guard
    // — without it this races `persist_theme_saves_the_live_mode_name`, which
    // then reads "paper-dark" instead of its own "random-light".
    let _g = crate::app::theme_test_guard();
    let mut app = CrewApp::default();
    let cfg = CrewConfig {
        font_size: 19.0,
        show_nav: false,
        ..CrewConfig::default()
    };
    // No renderer in tests: the font calls are skipped, but config is adopted
    // and a relayout/redraw is safe to request.
    app.apply_config(cfg);
    assert_eq!(app.config.font_size, 19.0);
    assert!(!app.config.show_nav);
}

#[test]
fn manual_family_change_disables_rotation() {
    // Guarded for the same reason: `apply_config` mutates the global theme
    // even though this test is only about the font.
    let _g = crate::app::theme_test_guard();
    let mut app = CrewApp::default();
    app.font_rotate.on = true;
    let mut cfg = app.config.clone();
    cfg.font_family = Some("Menlo".to_string());
    app.apply_config(cfg);
    assert!(!app.font_rotate.on, "explicit family pick stops rotation");
}

#[test]
fn pinning_a_family_says_it_stopped_rotation() {
    // It used to stop silently — and pinning your own font back is the natural
    // reaction to a rotated pick you dislike, so rotation died without a word
    // and the feature read as "/font random only works once".
    let _g = crate::app::theme_test_guard();
    let mut app = CrewApp::default();
    app.font_rotate.on = true;
    let mut cfg = app.config.clone();
    cfg.font_family = Some("Menlo".to_string());
    app.apply_config(cfg);
    let status = app.active_status().unwrap_or_default();
    assert!(status.contains("Menlo"), "{status}");
    assert!(status.contains("rotation off"), "{status}");
}

#[test]
fn a_config_apply_that_does_not_touch_the_family_says_nothing() {
    // Only a genuine pin should report; every Settings save re-applies the
    // config, and a status line that fires on each one is noise.
    let _g = crate::app::theme_test_guard();
    let mut app = CrewApp::default();
    app.font_rotate.on = true;
    let cfg = CrewConfig {
        font_size: 19.0,
        ..app.config.clone()
    };
    app.apply_config(cfg);
    assert!(
        !app.active_status()
            .unwrap_or_default()
            .contains("rotation off"),
        "a font-size change is not a pin"
    );
    assert!(app.font_rotate.on, "…and must not stop rotation");
}

/// An unrelated config touch must not re-roll the theme.
///
/// `apply_selection(Mode(..))` re-picks a theme AND restarts the 10-minute
/// clock. `apply_config` ran it on every apply — and a Cmd+= zoom, every
/// Settings save and every `/theme` all route through `apply_settings` →
/// `apply_config`. So the theme re-rolled whenever config was touched for any
/// reason, and a rotation cycle could never actually complete. It also made
/// rotation LOOK alive while the font (which has no such path) sat still.
#[test]
fn apply_config_does_not_reroll_an_already_active_rotation() {
    let _g = crate::app::theme_test_guard();
    crew_theme::apply_selection(
        crew_theme::Selection::Mode(crew_theme::RandomMode::Dark),
        1_000,
    );
    let picked = crew_theme::current_id();
    let mut app = CrewApp::default();

    // A config apply for an unrelated reason (here a font size), with the
    // same rotation mode already live.
    app.apply_config(CrewConfig {
        theme: Some("random-dark".into()),
        font_size: 19.0,
        ..CrewConfig::default()
    });

    assert!(crew_theme::is_random(), "still rotating");
    assert_eq!(
        crew_theme::current_id(),
        picked,
        "an unrelated config touch re-rolled the theme — the rotation's own \
         10-minute clock is the only thing that may change it"
    );
}

#[test]
fn apply_config_reconciles_random_mode() {
    let _g = crate::app::theme_test_guard();
    crew_theme::apply_selection(
        crew_theme::Selection::Fixed(crew_theme::ThemeId::PaperDark),
        0,
    );
    let mut app = CrewApp::default();

    // A saved `random` pin resumes rotation mode.
    app.apply_config(CrewConfig {
        theme: Some("random".into()),
        ..CrewConfig::default()
    });
    assert!(
        crew_theme::is_random(),
        "a saved `random` theme must resume rotation on apply"
    );

    // Applying a fixed theme (e.g. via the Settings pane) stops rotation and pins it.
    app.apply_config(CrewConfig {
        theme: Some("crt-green".into()),
        ..CrewConfig::default()
    });
    assert!(
        !crew_theme::is_random(),
        "picking a fixed theme in Settings must stop rotation"
    );
    assert_eq!(crew_theme::current_id(), crew_theme::ThemeId::CrtGreen);

    crew_theme::apply_selection(
        crew_theme::Selection::Fixed(crew_theme::ThemeId::PaperDark),
        0,
    );
}

#[test]
fn set_theme_cmd_switches_active_theme() {
    let _g = crate::app::theme_test_guard();
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
    let mut app = CrewApp::default();
    app.set_theme_cmd("paper-light");
    assert_eq!(crew_theme::current_id(), crew_theme::ThemeId::PaperLight);
    assert_eq!(app.config.theme.as_deref(), Some("paper-light"));
    // Unknown name leaves the active theme unchanged.
    app.set_theme_cmd("chartreuse");
    assert_eq!(crew_theme::current_id(), crew_theme::ThemeId::PaperLight);
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
}

#[test]
fn set_theme_cmd_clears_stale_look_overrides() {
    let _g = crate::app::theme_test_guard();
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
    let mut app = CrewApp::default();
    app.config.crt = Some(false);
    app.config.glass = "off".to_string();
    app.set_theme_cmd("crt");
    assert_eq!(
        app.config.crt, None,
        "a stale /crt pin must not outlive a theme switch"
    );
    assert_eq!(
        app.config.glass, "medium",
        "glass `off` returns to the frosted default on theme switch"
    );
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
}

#[test]
fn set_theme_cmd_keeps_a_deliberate_glass_strength() {
    let _g = crate::app::theme_test_guard();
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
    let mut app = CrewApp::default();
    app.config.crt = Some(true);
    app.config.glass = "high".to_string();
    app.set_theme_cmd("dark");
    assert_eq!(app.config.crt, None, "any /crt pin resets to auto");
    assert_eq!(
        app.config.glass, "high",
        "a chosen glass strength is taste, not a kill switch — it survives"
    );
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
}

#[test]
fn set_theme_cmd_random_enters_rotation_mode() {
    let _g = crate::app::theme_test_guard();
    crew_theme::apply_selection(
        crew_theme::Selection::Fixed(crew_theme::ThemeId::PaperDark),
        0,
    );
    let mut app = CrewApp::default();
    app.set_theme_cmd("random");
    assert!(crew_theme::is_random());
    // The `random` alias resolves to the canonical `dark` mode name.
    assert_eq!(app.config.theme.as_deref(), Some("dark"));

    // Switching to a fixed theme through this path also turns rotation off.
    app.set_theme_cmd("paper-light");
    assert!(!crew_theme::is_random());
    assert_eq!(crew_theme::current_id(), crew_theme::ThemeId::PaperLight);

    crew_theme::apply_selection(
        crew_theme::Selection::Fixed(crew_theme::ThemeId::PaperDark),
        0,
    );
}

#[test]
fn set_theme_cmd_reaches_the_modern_light_half() {
    // The input bar is where `/theme modern-light` is typed, and the whole
    // family is unreachable from it if this mode ever stops parsing: a build
    // that doesn't know the name answers "unknown theme", changes nothing and
    // persists nothing, which is indistinguishable from a theme that has no
    // effect. Pin the round trip — name in, LIGHT modern page out, name saved.
    let _g = crate::app::theme_test_guard();
    let mut app = CrewApp::default();
    app.set_theme_cmd("modern-light");
    assert_eq!(app.config.theme.as_deref(), Some("modern-light"));
    let id = crew_theme::current_id();
    assert!(
        !id.is_dark() && id.theme().modern.is_some(),
        "modern-light must land on a light modern palette, got {}",
        id.as_str()
    );
    crew_theme::apply_selection(
        crew_theme::Selection::Fixed(crew_theme::ThemeId::PaperDark),
        0,
    );
}

#[test]
fn an_unknown_theme_name_is_an_error_not_a_whisper() {
    // A name this build doesn't know changes nothing on screen, so the report
    // IS the whole feedback — at info level it was a three-second flash on the
    // input bar's border and was routinely missed. Error level also raises a
    // toast, and the log entry keeps it after the flash expires.
    let _g = crate::app::theme_test_guard();
    let mut app = CrewApp::default();
    let before = app.config.theme.clone();
    app.set_theme_cmd("modern-lite");
    assert_eq!(app.config.theme, before, "nothing is persisted");
    let last = app.log.last().expect("the miss is logged");
    assert_eq!(last.level, crate::applog::LogLevel::Error);
    assert!(
        last.text.contains("unknown theme 'modern-lite'"),
        "{last:?}"
    );
    // …and it names the modes that DO exist, modern-light among them.
    assert!(last.text.contains("modern-light"), "{last:?}");
    assert!(
        app.toasts.any_live(crate::anim::now_ms()),
        "an error status also steps onto the canvas as a toast"
    );
}

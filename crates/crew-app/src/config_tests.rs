use super::CrewConfig;

#[test]
fn default_values() {
    let cfg = CrewConfig::default();
    assert_eq!(cfg.font_size, 14.0);
    assert!(cfg.show_nav);
}

#[test]
fn notify_defaults_are_on() {
    let cfg = CrewConfig::default();
    assert!(cfg.notify);
    assert!(cfg.notify_agent_done);
    assert!(cfg.notify_bell);
    assert!(cfg.notify_exit);
    assert_eq!(cfg.notify_min_secs, 10);
    assert!(cfg.notify_patterns.is_empty());
}

#[test]
fn notify_min_secs_clamped() {
    // Zero is nonsensical (every quick command fires) → clamp up to 1.
    let cfg = CrewConfig::from_toml_str("notify_min_secs = 0\n");
    assert_eq!(cfg.notify_min_secs, 1);
    // Absurdly large → clamped down to an hour.
    let cfg = CrewConfig::from_toml_str("notify_min_secs = 99999\n");
    assert_eq!(cfg.notify_min_secs, 3600);
}

#[test]
fn usage_budgets_default_and_clamp() {
    let cfg = CrewConfig::from_toml_str("");
    assert_eq!(cfg.usage_budget_5h, 5_000_000);
    assert_eq!(cfg.usage_budget_7d, 25_000_000);
    let cfg = CrewConfig::from_toml_str("usage_budget_5h = 1\n");
    assert_eq!(cfg.usage_budget_5h, 10_000);
}

#[test]
fn notify_patterns_drop_blanks() {
    let cfg = CrewConfig::from_toml_str("notify_patterns = [\"error\", \"\", \"done\"]\n");
    assert_eq!(
        cfg.notify_patterns,
        vec!["error".to_string(), "done".to_string()]
    );
}

#[test]
fn clamped_out_of_range() {
    let cfg = CrewConfig {
        font_size: 99.0,
        nav_width: 9.0,
        show_nav: true,
        font_family: None,
        accent: None,
        maximized: false,
        last_dir: None,
        win_w: Some(50.0),
        win_h: Some(50.0),
        ..CrewConfig::default()
    }
    .clamped();
    assert_eq!(cfg.font_size, 32.0);
    assert_eq!(cfg.nav_width, 160.0);
    assert!(cfg.show_nav);
    // window size is clamped up to sane minimums
    assert_eq!(cfg.win_w, Some(400.0));
    assert_eq!(cfg.win_h, Some(300.0));
}

#[test]
fn from_toml_partial() {
    let cfg = CrewConfig::from_toml_str("font_size = 25.0\n");
    assert_eq!(cfg.font_size, 25.0);
    assert_eq!(cfg.nav_width, 210.0);
    assert!(cfg.show_nav);
}

#[test]
fn from_toml_garbage() {
    let cfg = CrewConfig::from_toml_str("garbage {{{");
    assert_eq!(cfg, CrewConfig::default());
}

#[test]
fn round_trip() {
    let c = CrewConfig {
        font_size: 20.0,
        nav_width: 200.0,
        show_nav: true,
        font_family: Some("Menlo".to_string()),
        font_random: false,
        accent: Some("#112233".to_string()),
        maximized: true,
        last_dir: Some("/tmp".to_string()),
        win_w: Some(1024.0),
        win_h: Some(768.0),
        notify: true,
        notify_agent_done: false,
        notify_bell: true,
        notify_exit: false,
        notify_min_secs: 30,
        notify_patterns: vec!["error".to_string(), "done".to_string()],
        theme: Some("paper-light".to_string()),
        paper_texture: false,
        paper_grain: 0.5,
        crt: Some(true),
        font_weight: 700,
        usage_budget_5h: 1_000_000,
        usage_budget_7d: 12_000_000,
    };
    assert_eq!(CrewConfig::from_toml_str(&c.to_toml_str()), c);
}

#[test]
fn line_height() {
    let cfg = CrewConfig::default();
    assert!((cfg.line_height() - 17.5).abs() < 1e-6);
}

#[test]
fn accent_rgb_parses_or_falls_back() {
    let _g = crate::app::theme_test_guard();
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
    // Unset → active theme default.
    assert_eq!(
        CrewConfig::default().accent_rgb(),
        crew_theme::PAPER_DARK.accent_default
    );
    // Valid hex → parsed.
    let cfg = CrewConfig::from_toml_str("accent = \"#102030\"\n");
    assert_eq!(cfg.accent_rgb(), (0x10, 0x20, 0x30));
    // Invalid hex → theme default (not a panic).
    let bad = CrewConfig::from_toml_str("accent = \"not-a-color\"\n");
    assert_eq!(bad.accent_rgb(), crew_theme::PAPER_DARK.accent_default);
}

#[test]
fn empty_accent_clamped_to_none() {
    let cfg = CrewConfig::from_toml_str("accent = \"\"\n");
    assert_eq!(cfg.accent, None);
}

#[test]
fn theme_id_parses_or_defaults() {
    assert_eq!(
        CrewConfig::default().theme_id(),
        crew_theme::ThemeId::PaperDark
    );
    let light = CrewConfig::from_toml_str("theme = \"paper-light\"\n");
    assert_eq!(light.theme_id(), crew_theme::ThemeId::PaperLight);
    let bad = CrewConfig::from_toml_str("theme = \"chartreuse\"\n");
    assert_eq!(bad.theme_id(), crew_theme::ThemeId::PaperDark);
}

#[test]
fn font_random_round_trips_and_defaults_off() {
    let cfg = CrewConfig::from_toml_str("");
    assert!(!cfg.font_random);
    let cfg = CrewConfig::from_toml_str("font_random = true\n");
    assert!(cfg.font_random);
    assert!(cfg.clamped().font_random, "clamped() must carry the flag");
}

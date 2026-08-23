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
fn model_recents_default_empty_and_round_trip() {
    let c: CrewConfig = toml::from_str("").unwrap();
    assert!(c.model_recents.is_empty()); // old config files still load
    let c = CrewConfig {
        model_recents: vec!["qwen-max".into()],
        ..c
    };
    let back: CrewConfig = toml::from_str(&toml::to_string(&c).unwrap()).unwrap();
    assert_eq!(back.model_recents, vec!["qwen-max".to_string()]);
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
        last_seen_version: None,
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
        theme_dark: Some("crt".to_string()),
        theme_light: Some("sepia-light".to_string()),
        paper_texture: false,
        paper_grain: 0.5,
        crt: Some(true),
        glass: "high".to_string(),
        motion: "full".to_string(),
        window_opacity: 0.85,
        font_weight: 700,
        font_smooth: 120,
        usage_budget_5h: 1_000_000,
        usage_budget_7d: 12_000_000,
        model_recents: vec!["qwen-max".to_string()],
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
fn theme_selection_defaults_fresh_installs_to_os_following_auto() {
    use crew_theme::{RandomMode, Selection, ThemeId};
    // No saved theme: follow the OS. This is the fresh-install default, and
    // both startup (handler) and apply_config resolve through this one fn.
    assert_eq!(
        CrewConfig::default().theme_selection(),
        Selection::Mode(RandomMode::Auto)
    );
    assert_eq!(CrewConfig::default().theme_label(), "auto");
    // A saved selection — mode or palette — keeps exactly its meaning:
    // an upgrade never hijacks an explicit pick into following the OS.
    let dark = CrewConfig::from_toml_str("theme = \"dark\"\n");
    assert_eq!(dark.theme_selection(), Selection::Mode(RandomMode::Dark));
    let pinned = CrewConfig::from_toml_str("theme = \"paper-light\"\n");
    assert_eq!(
        pinned.theme_selection(),
        Selection::Fixed(ThemeId::PaperLight)
    );
    // A garbage value keeps its historical fallback (fixed default palette),
    // not auto — a broken string is not an intent to follow the OS.
    let bad = CrewConfig::from_toml_str("theme = \"chartreuse\"\n");
    assert_eq!(bad.theme_selection(), Selection::Fixed(ThemeId::PaperDark));
}

#[test]
fn auto_pool_pairing_round_trips_and_survives_clamped() {
    use crew_theme::{RandomMode, Selection, ThemeId};
    // clamped() must carry both fields (the last_seen_version lesson: a
    // literal in the rebuild silently resets the field on every load).
    let cfg = CrewConfig::from_toml_str("theme_dark = \"crt\"\ntheme_light = \"paper-light\"\n")
        .clamped();
    assert_eq!(cfg.theme_dark.as_deref(), Some("crt"));
    assert_eq!(cfg.theme_light.as_deref(), Some("paper-light"));
    let (d, l) = cfg.auto_pool_selections();
    assert_eq!(d, Some(Selection::Mode(RandomMode::Crt)));
    assert_eq!(l, Some(Selection::Fixed(ThemeId::PaperLight)));
    // `auto` can't serve as its own side, and garbage falls back to the
    // built-in pool for that appearance.
    let bad = CrewConfig::from_toml_str("theme_dark = \"auto\"\ntheme_light = \"nope\"\n");
    assert_eq!(bad.auto_pool_selections(), (None, None));
    // Unset stays unset — the built-in dark/light paper pairing.
    assert_eq!(CrewConfig::default().auto_pool_selections(), (None, None));
}

#[test]
fn font_random_round_trips_and_defaults_off() {
    let cfg = CrewConfig::from_toml_str("");
    assert!(!cfg.font_random);
    let cfg = CrewConfig::from_toml_str("font_random = true\n");
    assert!(cfg.font_random);
    assert!(cfg.clamped().font_random, "clamped() must carry the flag");
}

#[test]
fn a_fully_opaque_window_does_not_ask_to_be_transparent() {
    // THE TITLE-BAR BUG. `handler.rs` requests `.with_transparent(true)` at
    // creation so the Opacity % setting can take effect without a restart —
    // but on macOS that sets `NSWindow.isOpaque = false` for good, and the
    // OS-drawn title bar composites against whatever is behind the window.
    // Crew's own frame is opaque at 1.0, so the panes looked right while the
    // title bar showed the desktop through it: the title bar is not part of
    // the frame crew draws, so "nothing crew draws leaves alpha below 1"
    // never covered it.
    assert!(!super::wants_window_transparency(
        super::default_window_opacity()
    ));
    assert!(!super::wants_window_transparency(1.0));
}

#[test]
fn any_opacity_below_one_asks_to_be_transparent() {
    // The other direction matters just as much: gating this on the setting is
    // only correct if translucency still works when it is actually wanted.
    assert!(super::wants_window_transparency(0.99));
    assert!(super::wants_window_transparency(super::MIN_WINDOW_OPACITY));
}

#[test]
fn reset_look_overrides_clears_the_pins_that_gut_a_theme() {
    // THE "CRT THEME IS JUST A DARK THEME" BUG: a persisted `crt = false` +
    // `glass = "off"` from months ago silently disabled the entire CRT
    // post-process and glass sheet on every later theme switch.
    let mut cfg = CrewConfig {
        crt: Some(false),
        glass: "off".to_string(),
        ..Default::default()
    };
    assert!(cfg.reset_look_overrides());
    assert_eq!(cfg.crt, None, "the /crt pin returns to follow-the-theme");
    assert_eq!(
        cfg.glass, "medium",
        "glass off returns to the frosted default"
    );
}

#[test]
fn reset_look_overrides_keeps_a_chosen_glass_strength() {
    let mut cfg = CrewConfig {
        glass: "high".to_string(),
        ..Default::default()
    };
    assert!(!cfg.reset_look_overrides(), "nothing look-killing to clear");
    assert_eq!(cfg.glass, "high", "low/high are taste, not kill switches");
}

#[test]
fn reset_look_overrides_reports_when_nothing_changed() {
    let mut cfg = CrewConfig::default();
    assert!(
        !cfg.reset_look_overrides(),
        "defaults have no pins; claiming a change would trigger useless saves"
    );
    cfg.crt = Some(true);
    assert!(
        cfg.reset_look_overrides(),
        "a /crt on pin also resets to auto"
    );
    assert_eq!(cfg.crt, None);
}

#[test]
fn load_keeps_the_last_seen_version() {
    // `load()` runs `clamped()`, which used to rebuild the struct with
    // `last_seen_version: None` — so every launch read as a first run: the
    // "updated to crew X" note never fired and version-gated migrations
    // (e.g. the 0.12.6 crt/glass-pin heal) never ran.
    let cfg = CrewConfig::from_toml_str("last_seen_version = \"0.12.5\"\n");
    assert_eq!(cfg.last_seen_version.as_deref(), Some("0.12.5"));
}

use crate::app::CrewApp;

#[test]
fn allowed_pool_keeps_only_allowlisted() {
    let installed = vec![
        "Courier New".into(),
        "MonoLisa".into(),
        "Some Random Face".into(),
        "JetBrainsMono NF".into(),
    ];
    let pool = super::allowed_pool(installed);
    assert!(pool.contains(&"MonoLisa".to_string()));
    assert!(pool.contains(&"JetBrainsMono NF".to_string()));
    assert!(
        !pool.iter().any(|f| f == "Courier New"),
        "Courier is not allowlisted"
    );
    assert!(!pool.iter().any(|f| f == "Some Random Face"));
}

#[test]
fn allowed_pool_falls_back_when_none_allowlisted() {
    let installed = vec!["Some Random Face".into(), "Another Face".into()];
    let pool = super::allowed_pool(installed.clone());
    assert_eq!(pool, installed, "no allowlisted face → keep the full set");
}

#[test]
fn parses_and_clamps_to_range() {
    // A font-SIZE test needs the theme guard: `set_font` persists through
    // `apply_settings` → `apply_config`, which pins a fixed theme app-wide
    // and clears the random MODE. Without this it races
    // `chataction::persist_theme_saves_the_live_mode_name`, which then
    // reads "paper-dark" instead of its own "random-light".
    let _g = crate::app::theme_test_guard();
    let mut app = CrewApp::default();
    app.set_font_cmd("18");
    assert_eq!(app.config.font_size, 18.0);
    app.set_font_cmd("5"); // below min → clamps up
    assert_eq!(app.config.font_size, 12.0);
    app.set_font_cmd("999"); // above max → clamps down
    assert_eq!(app.config.font_size, 32.0);
}

#[test]
fn rejects_non_number_without_changing_size() {
    let mut app = CrewApp::default();
    let before = app.config.font_size;
    app.set_font_cmd("big");
    assert_eq!(app.config.font_size, before);
    assert!(app.active_status().is_some());
}

#[test]
fn font_random_arg_enables_rotation_or_reports_thin_pool() {
    let mut app = CrewApp::default();
    app.set_font_cmd("random");
    // Headless default app has no renderer → pool scan yields nothing →
    // rotation must stay off with the thin-pool report.
    assert!(!app.font_rotate.on);
    assert!(app.active_status().is_some());
}

/// Seed the pool the way a live renderer scan would. Headless there is no
/// renderer, so `font_pool` caches an EMPTY vec and `pick` returns None —
/// which is exactly why the enabled path had never been exercised.
fn rotating_app() -> CrewApp {
    let mut app = CrewApp::default();
    app.font_rotate.pool = Some(vec!["Menlo".into(), "Monaco".into()]);
    app.font_rotate.on = true;
    app.font_rotate.current = Some("Menlo".into());
    app.font_rotate.last_ms = 0;
    app
}

#[test]
fn a_theme_change_applies_that_themes_font() {
    let _g = crate::app::theme_test_guard();
    let mut app = rotating_app();
    // Pool holds a face the CRT themes prefer and one they don't.
    app.font_rotate.pool = Some(vec!["Monaco".into(), "IBM Plex Mono".into()]);
    crew_theme::apply_selection(
        crew_theme::Selection::Fixed(crew_theme::ThemeId::CrtGreen),
        0,
    );
    assert!(app.tick_theme_font(), "a theme change must apply its font");
    assert_eq!(app.font_rotate.current.as_deref(), Some("Monaco"));

    // Switching theme switches font.
    crew_theme::apply_selection(
        crew_theme::Selection::Fixed(crew_theme::ThemeId::PaperDark),
        0,
    );
    assert!(app.tick_theme_font());
    assert_eq!(app.font_rotate.current.as_deref(), Some("IBM Plex Mono"));
}

#[test]
fn the_theme_font_is_applied_once_not_every_tick() {
    // poll runs at ~62 Hz; re-applying a family every tick would churn the
    // glyph atlas for nothing.
    let _g = crate::app::theme_test_guard();
    let mut app = rotating_app();
    app.font_rotate.pool = Some(vec!["Monaco".into()]);
    crew_theme::apply_selection(
        crew_theme::Selection::Fixed(crew_theme::ThemeId::CrtGreen),
        0,
    );
    assert!(app.tick_theme_font(), "first tick applies");
    assert!(!app.tick_theme_font(), "second tick must be a no-op");
    assert!(!app.tick_theme_font());
}

#[test]
fn a_rotation_pick_survives_later_ticks_of_an_unchanged_theme() {
    // The heart of the temporal rule: a theme change sets the font, but
    // the NEXT rotation overrides it and must then stick. If
    // `tick_theme_font` re-applied on every tick rather than only on a
    // change, it would stamp the theme's font back ~62 times a second and
    // the rotation could never win — "both" would collapse to "theme
    // only". The `is_applied_once` test above cannot see this: its second
    // guard (already-showing-it) masks a missing change check.
    let _g = crate::app::theme_test_guard();
    let mut app = rotating_app();
    app.font_rotate.pool = Some(vec!["Monaco".into(), "IBM Plex Mono".into()]);
    crew_theme::apply_selection(
        crew_theme::Selection::Fixed(crew_theme::ThemeId::CrtGreen),
        0,
    );
    app.tick_theme_font();
    assert_eq!(app.font_rotate.current.as_deref(), Some("Monaco"));

    // 10 minutes on, the rotation picks the other face.
    app.font_rotate.last_ms = 0;
    assert!(app.tick_font_rotation(crew_theme::ROTATE_MS));
    assert_eq!(app.font_rotate.current.as_deref(), Some("IBM Plex Mono"));

    // Many more ticks with the theme unchanged: the pick must stand.
    for _ in 0..5 {
        assert!(!app.tick_theme_font(), "no theme change — nothing to do");
    }
    assert_eq!(
        app.font_rotate.current.as_deref(),
        Some("IBM Plex Mono"),
        "the theme stamped its font back over a live rotation"
    );
}

#[test]
fn a_theme_whose_fonts_are_all_missing_changes_nothing() {
    // A family that isn't installed makes fontdb substitute a PROPORTIONAL
    // face, and cell rounding then mangles every glyph — so an
    // unresolvable preference must leave the font alone, not guess.
    let _g = crate::app::theme_test_guard();
    let mut app = rotating_app();
    app.font_rotate.pool = Some(vec!["Nothing The Theme Wants".into()]);
    app.font_rotate.current = Some("Nothing The Theme Wants".into());
    crew_theme::apply_selection(
        crew_theme::Selection::Fixed(crew_theme::ThemeId::CrtGreen),
        0,
    );
    assert!(!app.tick_theme_font(), "must not apply an absent family");
    assert_eq!(
        app.font_rotate.current.as_deref(),
        Some("Nothing The Theme Wants"),
        "font must be left exactly as it was"
    );
}

#[test]
fn the_theme_font_beats_the_rotation_on_a_shared_tick() {
    // Both hang off the same 10-minute clock, so they fire together on
    // every rotation. `poll_panes` runs the theme font last for exactly
    // this: the theme wins the tie.
    let _g = crate::app::theme_test_guard();
    let mut app = rotating_app();
    app.font_rotate.pool = Some(vec!["Monaco".into(), "IBM Plex Mono".into()]);
    app.font_rotate.current = Some("IBM Plex Mono".into());
    crew_theme::apply_selection(
        crew_theme::Selection::Fixed(crew_theme::ThemeId::CrtGreen),
        0,
    );

    app.tick_font_rotation(crew_theme::ROTATE_MS); // rotation picks something
    app.tick_theme_font(); // …the theme overrides it
    assert_eq!(
        app.font_rotate.current.as_deref(),
        Some("Monaco"),
        "the theme's font must land on top of the rotation's pick"
    );
}

#[test]
fn a_due_rotation_applies_a_new_family() {
    // The wiring test that did not exist: `due` and `pick` were covered in
    // isolation, so nothing proved a due rotation reaches the renderer.
    let mut app = rotating_app();
    let now = crew_theme::ROTATE_MS;
    assert!(app.tick_font_rotation(now), "a due rotation must apply");
    assert_eq!(app.font_rotate.current.as_deref(), Some("Monaco"));
    assert_eq!(app.font_rotate.last_ms, now, "clock must restamp");
}

#[test]
fn rotation_does_not_fire_before_the_clock_elapses() {
    let mut app = rotating_app();
    assert!(!app.tick_font_rotation(crew_theme::ROTATE_MS - 1));
    assert_eq!(
        app.font_rotate.current.as_deref(),
        Some("Menlo"),
        "family must not change early"
    );
}

#[test]
fn rotation_does_not_fire_while_off() {
    let mut app = rotating_app();
    app.font_rotate.on = false;
    assert!(!app.tick_font_rotation(crew_theme::ROTATE_MS));
    assert_eq!(app.font_rotate.current.as_deref(), Some("Menlo"));
}

#[test]
fn rotation_keeps_firing_on_each_subsequent_clock() {
    // One rotation working is not the reported symptom — "sets a font once
    // and stops" is. Prove the SECOND rotation lands too.
    let mut app = rotating_app();
    assert!(app.tick_font_rotation(crew_theme::ROTATE_MS));
    assert_eq!(app.font_rotate.current.as_deref(), Some("Monaco"));
    assert!(
        app.tick_font_rotation(crew_theme::ROTATE_MS * 2),
        "the second rotation must fire too"
    );
    assert_eq!(
        app.font_rotate.current.as_deref(),
        Some("Menlo"),
        "two-font pool must swing back, not stick"
    );
}

#[test]
fn no_arg_report_mentions_rotation_state() {
    let mut app = CrewApp::default();
    app.set_font_cmd("");
    let s = app.active_status().unwrap();
    assert!(s.contains("font size"), "{s}");
}

#[test]
fn font_random_while_rotating_toggles_off_and_restores_pinned() {
    let mut app = CrewApp::default();
    app.config.font_family = Some("Pinned Mono".to_string());
    app.font_rotate.on = true;
    app.font_rotate.current = Some("Rotated Mono".to_string());
    app.config.font_random = true;
    app.set_font_cmd("random");
    assert!(
        !app.font_rotate.on,
        "second /font random turns rotation off"
    );
    assert!(app.font_rotate.current.is_none());
    assert!(!app.config.font_random);
    assert_eq!(app.config.font_family.as_deref(), Some("Pinned Mono"));
    let s = app.active_status().unwrap();
    assert!(s.contains("rotation off"), "{s}");
}

#[test]
fn rotation_never_touches_the_pinned_config_family() {
    // The feature's core safety property: a rotated pick lives on
    // font_rotate.current ONLY, so unrelated config.save() calls (the
    // resize settle, /theme) can never persist it.
    let mut app = CrewApp::default();
    app.config.font_family = Some("Pinned Mono".to_string());
    app.apply_rotated_family("Rotated Mono".to_string());
    assert_eq!(app.config.font_family.as_deref(), Some("Pinned Mono"));
    assert_eq!(app.font_rotate.current.as_deref(), Some("Rotated Mono"));
}

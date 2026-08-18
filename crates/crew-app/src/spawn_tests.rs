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
fn the_retired_modern_names_still_land_on_their_own_appearance() {
    // `modern` / `modern-light` were typed at this bar for two releases and
    // live in saved configs. They are no longer modes — the palettes rotate
    // inside `dark` / `light` — but a name that stops parsing answers "unknown
    // theme" and changes nothing, which is indistinguishable from a theme with
    // no effect. So they still resolve, to the pool that swallowed them, and
    // crucially never across the appearance line.
    let _g = crate::app::theme_test_guard();
    let mut app = CrewApp::default();
    app.set_theme_cmd("modern-light");
    assert_eq!(app.config.theme.as_deref(), Some("light"));
    assert!(
        !crew_theme::current_id().is_dark(),
        "modern-light must still open a LIGHT page, got {}",
        crew_theme::current_id().as_str()
    );
    app.set_theme_cmd("modern");
    assert_eq!(app.config.theme.as_deref(), Some("dark"));
    assert!(
        crew_theme::current_id().is_dark(),
        "modern must still open a DARK page, got {}",
        crew_theme::current_id().as_str()
    );
    crew_theme::apply_selection(
        crew_theme::Selection::Fixed(crew_theme::ThemeId::PaperDark),
        0,
    );
}

#[test]
fn the_dark_and_light_pools_rotate_the_modern_palettes_too() {
    // The consolidation as the user meets it: pick `dark`, sit through
    // rotations, and the Gemini/Codex palettes come up alongside the paper
    // ones — they are no longer a separate theme you have to go and choose.
    let _g = crate::app::theme_test_guard();
    let mut app = CrewApp::default();
    // `set_theme_cmd` stamps the rotation clock with the WALL clock, so the
    // ticks have to start from there — counting up from zero is in the past
    // and `tick_random` (rightly) never fires.
    let base = crate::chattime::unix_now_ms();
    for (name, want_dark) in [("dark", true), ("light", false)] {
        app.set_theme_cmd(name);
        let mut seen_modern = crew_theme::current_id().theme().modern.is_some();
        for tick in 1..=40u64 {
            // A second past each 10-minute mark: the clock was stamped a hair
            // after `base`, so landing exactly on it falls just short.
            assert!(
                crew_theme::tick_random(base + tick * (crew_theme::ROTATE_MS + 1_000)),
                "the rotation clock never advanced"
            );
            let id = crew_theme::current_id();
            assert_eq!(id.is_dark(), want_dark, "{name} rotated off its own side");
            assert!(!id.is_crt(), "{name} rotated onto a tube: {}", id.as_str());
            seen_modern |= id.theme().modern.is_some();
        }
        assert!(seen_modern, "no modern palette ever came up in `{name}`");
    }
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
    // …and it names the modes that DO exist, all four of them.
    for mode in ["dark", "light", "crt", "auto"] {
        assert!(last.text.contains(mode), "{mode} missing from {last:?}");
    }
    assert!(
        app.toasts.any_live(crate::anim::now_ms()),
        "an error status also steps onto the canvas as a toast"
    );
}

/// The shells a Windows pane opens must actually be spawnable on the host —
/// this is the regression that made the platform build but not run: every
/// pane tried `/bin/sh`, failed, and reported "couldn't open shell".
/// Spawning them for real (rather than asserting a string) is the only form
/// of this test that would have caught it.
#[cfg(windows)]
#[test]
fn the_windows_shells_exist_and_start() {
    use crate::pane::spawn_pane;
    use crew_term::GridSize;

    let grid = GridSize { cols: 40, rows: 10 };
    for shell in [super::preferred_shell(), super::fallback_shell()] {
        let pane = spawn_pane(&shell, &shell, grid, None);
        assert!(pane.is_ok(), "could not open a pane with {shell}");
    }
}

/// `-l` is a Unix login-shell flag; PowerShell and cmd.exe reject it, so the
/// default pane spawn must not carry it on Windows. Covered by the spawn above
/// (it goes through the same `spawn_pane`), asserted here as intent.
#[cfg(unix)]
#[test]
fn the_unix_fallback_shell_is_bourne() {
    assert_eq!(super::fallback_shell(), "/bin/sh");
    assert!(std::path::Path::new(&super::fallback_shell()).exists());
}

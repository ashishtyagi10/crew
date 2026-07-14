use super::*;

/// Serialises tests that mutate the process-wide CURRENT.
fn guard() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

#[test]
fn default_is_paper_dark() {
    let _g = guard();
    // At rest (no set_theme yet in this process) the default id is PaperDark.
    // We don't assert on a possibly-mutated global; just the mapping.
    assert_eq!(ThemeId::from_u8(0), ThemeId::PaperDark);
}

#[test]
fn id_string_round_trip() {
    for id in ALL_THEMES {
        assert_eq!(ThemeId::from_name(id.as_str()), Some(id));
    }
    assert_eq!(ThemeId::from_name("nope"), None);
    assert_eq!(
        ThemeId::from_name("  paper-light "),
        Some(ThemeId::PaperLight)
    );
    assert_eq!(ThemeId::from_name("crt-green"), Some(ThemeId::CrtGreen));
}

#[test]
fn next_cycles_through_all_and_wraps() {
    // Every theme steps to another, and stepping the whole ring returns home.
    let mut id = ThemeId::PaperDark;
    for _ in 0..ALL_THEMES.len() {
        id = id.next();
    }
    assert_eq!(id, ThemeId::PaperDark);
    assert_eq!(ThemeId::CrtViolet.next(), ThemeId::PaperDark); // last wraps to first
}

#[test]
fn set_then_current_round_trips() {
    let _g = guard();
    set_theme(ThemeId::PaperLight);
    assert_eq!(current_id(), ThemeId::PaperLight);
    assert_eq!(theme().page_bg, PAPER_LIGHT.page_bg);
    set_theme(ThemeId::PaperDark);
    assert_eq!(current_id(), ThemeId::PaperDark);
}

#[test]
fn no_preset_uses_pure_black_or_white() {
    for id in ALL_THEMES {
        let t = id.theme();
        let mut all = vec![
            t.page_bg,
            t.ink,
            t.text_muted,
            t.term_fg,
            t.term_bg,
            t.border_normal,
            t.border_focused,
            t.legend_off,
            t.accent_default,
            t.status_fg,
            t.broadcast,
            t.activity,
            t.bell,
            t.dim,
            t.placeholder,
            t.hint_fg,
            t.find_hl_bg,
        ];
        all.extend_from_slice(&t.ansi);
        for c in all {
            assert_ne!(c, (0, 0, 0), "pure black found in a preset");
            assert_ne!(c, (255, 255, 255), "pure white found in a preset");
        }
    }
}

#[test]
fn term_bg_equals_page_bg() {
    for id in ALL_THEMES {
        let t = id.theme();
        assert_eq!(t.term_bg, t.page_bg);
    }
}

#[test]
fn term_fg_bg_have_contrast() {
    // crude luminance gap so default text is never near-invisible.
    for id in ALL_THEMES {
        let t = id.theme();
        let lum = |c: (u8, u8, u8)| c.0 as i32 + c.1 as i32 + c.2 as i32;
        assert!((lum(t.term_fg) - lum(t.term_bg)).abs() > 200);
    }
}

#[test]
fn random_pick_never_returns_current_and_is_deterministic() {
    let _g = guard();
    for current in ALL_THEMES {
        for seed in [0u64, 1, 2, 42, 1_000, 600_000, u64::MAX, 123_456_789] {
            let picked = random_pick(current, seed, true);
            assert_ne!(picked, current, "seed {seed} picked the current theme");
            // Same seed -> same pick (determinism).
            assert_eq!(random_pick(current, seed, true), picked);
        }
    }
    // Varying the seed actually varies the pick (not a constant function).
    let current = ThemeId::PaperDark;
    let picks: Vec<ThemeId> = (0u64..20).map(|s| random_pick(current, s, true)).collect();
    assert!(
        picks.iter().any(|&p| p != picks[0]),
        "random_pick looks constant across seeds: {picks:?}"
    );
}

#[test]
fn tick_random_fires_at_rotate_ms_when_on() {
    let _g = guard();
    set_theme(ThemeId::PaperDark);
    MODE.store(1, Ordering::Relaxed);
    ROTATED_MS.store(0, Ordering::Relaxed);
    assert!(!tick_random(ROTATE_MS - 1));
    assert_eq!(current_id(), ThemeId::PaperDark);
    let before = current_id();
    assert!(tick_random(ROTATE_MS));
    assert_ne!(current_id(), before);

    // Random OFF: never fires, no matter how much time has passed.
    apply_selection(Selection::Fixed(ThemeId::PaperDark), 0);
    assert!(!tick_random(10_000_000));
    set_theme(ThemeId::PaperDark);
}

#[test]
fn cycle_next_walks_all_themes_then_random_then_wraps() {
    let _g = guard();
    apply_selection(Selection::Fixed(ThemeId::PaperDark), 0);
    // Starting at paper-dark, each call steps to the next fixed theme...
    for want in [
        "paper-light",
        "sepia-dark",
        "sepia-light",
        "midnight-ink",
        "graphite",
        "coldpress-gray",
        "salmon-broadsheet",
        "ivory-ledger",
        "crt-green",
        "crt-amber",
        "crt-blue",
        "crt-violet",
    ] {
        assert_eq!(cycle_next(1), want);
    }
    // ...then from the last fixed theme it enters random-dark...
    assert_eq!(cycle_next(5), "random-dark");
    assert!(is_random());
    // ...then random-light...
    assert_eq!(cycle_next(6), "random-light");
    assert!(is_random());
    // ...then auto...
    assert_eq!(cycle_next(7), "auto");
    assert!(is_random());
    // ...and from auto it wraps back to the first fixed theme, off.
    assert_eq!(cycle_next(8), "paper-dark");
    assert!(!is_random());
    assert_eq!(current_id(), ThemeId::PaperDark);
    apply_selection(Selection::Fixed(ThemeId::PaperDark), 0);
}

#[test]
fn u8_mapping_round_trips_all_ids() {
    // Persistence mapping: every id survives as_u8 → from_u8 (via the
    // set_theme/current_id atomics); the new ids extend the mapping
    // without renumbering the original nine.
    let _g = guard();
    for id in ALL_THEMES {
        set_theme(id);
        assert_eq!(current_id(), id, "{} lost by u8 round-trip", id.as_str());
    }
    assert_eq!(ThemeId::from_u8(5), ThemeId::SepiaDark);
    assert_eq!(ThemeId::from_u8(6), ThemeId::MidnightInk);
    assert_eq!(ThemeId::from_u8(7), ThemeId::Graphite);
    assert_eq!(ThemeId::from_u8(8), ThemeId::CrtViolet);
    assert_eq!(ThemeId::from_u8(9), ThemeId::SepiaLight);
    assert_eq!(ThemeId::from_u8(10), ThemeId::SalmonBroadsheet);
    assert_eq!(ThemeId::from_u8(11), ThemeId::ColdpressGray);
    assert_eq!(ThemeId::from_u8(12), ThemeId::IvoryLedger);
    set_theme(ThemeId::PaperDark);
}

#[test]
fn contrast_thresholds() {
    let cr = contrast_ratio;
    for id in ALL_THEMES {
        let name = id.as_str();
        let t = id.theme();
        let bg = t.page_bg;
        let tbg = t.term_bg;

        assert!(
            cr(t.ink, bg) >= 10.0,
            "{name}: ink vs page_bg = {:.3} (need >= 10.0)",
            cr(t.ink, bg)
        );
        assert!(
            cr(t.term_fg, tbg) >= 10.0,
            "{name}: term_fg vs term_bg = {:.3} (need >= 10.0)",
            cr(t.term_fg, tbg)
        );
        assert!(
            cr(t.text_muted, bg) >= 7.0,
            "{name}: text_muted vs page_bg = {:.3} (need >= 7.0)",
            cr(t.text_muted, bg)
        );
        assert!(
            cr(t.legend_off, bg) >= 3.0,
            "{name}: legend_off vs page_bg = {:.3} (need >= 3.0)",
            cr(t.legend_off, bg)
        );
        assert!(
            cr(t.hint_fg, bg) >= 2.5,
            "{name}: hint_fg vs page_bg = {:.3} (need >= 2.5)",
            cr(t.hint_fg, bg)
        );
        assert!(
            cr(t.placeholder, bg) >= 2.3,
            "{name}: placeholder vs page_bg = {:.3} (need >= 2.3)",
            cr(t.placeholder, bg)
        );
        assert!(
            cr(t.accent_default, bg) >= 3.0,
            "{name}: accent_default vs page_bg = {:.3} (need >= 3.0)",
            cr(t.accent_default, bg)
        );
        assert!(
            cr(t.border_focused, bg) >= 2.2,
            "{name}: border_focused vs page_bg = {:.3} (need >= 2.2)",
            cr(t.border_focused, bg)
        );
        assert!(
            cr(t.border_normal, bg) >= 1.45,
            "{name}: border_normal vs page_bg = {:.3} (need >= 1.45)",
            cr(t.border_normal, bg)
        );
        // ANSI terminal colours (skip slots 0, 7, 8, 15 = blacks and whites)
        for i in [1usize, 2, 3, 4, 5, 6, 9, 10, 11, 12, 13, 14] {
            assert!(
                cr(t.ansi[i], tbg) >= 3.0,
                "{name}: ansi[{i}] {:?} vs term_bg = {:.3} (need >= 3.0)",
                t.ansi[i],
                cr(t.ansi[i], tbg)
            );
        }
    }
}

#[test]
fn dark_flag_matches_page_bg_luminance() {
    // The `dark` field is design data, but it may never contradict the
    // palette: WCAG relative luminance of page_bg < 0.5 ⇔ dark.
    let lin = |c: u8| -> f32 {
        let x = c as f32 / 255.0;
        if x <= 0.03928 {
            x / 12.92
        } else {
            ((x + 0.055) / 1.055).powf(2.4)
        }
    };
    for id in ALL_THEMES {
        let t = id.theme();
        let lum = 0.2126 * lin(t.page_bg.0) + 0.7152 * lin(t.page_bg.1) + 0.0722 * lin(t.page_bg.2);
        assert_eq!(
            t.dark,
            lum < 0.5,
            "{}: dark={} but page_bg luminance={lum:.3}",
            id.as_str(),
            t.dark
        );
    }
}

#[test]
fn grain_is_newsprint_on_every_theme() {
    // 1.2 across the board (not the historical 3.0): gamma-space blending
    // (v0.5.58) modulates encoded values, which reads much stronger than the
    // old linear-space grain. Dark themes now match light (was 1.0) so the
    // newspaper texture reads on the dark pages too — the shader's dark
    // absolute term carries it (see paperbg.wgsl).
    for id in ALL_THEMES {
        let t = id.theme();
        assert_eq!(t.grain, 1.2, "{}: grain", id.as_str());
    }
}

#[test]
fn parse_selection_names_modes_and_alias() {
    assert_eq!(
        parse_selection("paper-light"),
        Some(Selection::Fixed(ThemeId::PaperLight))
    );
    assert_eq!(
        parse_selection(" random-dark "),
        Some(Selection::Mode(RandomMode::Dark))
    );
    assert_eq!(
        parse_selection("Random-Light"),
        Some(Selection::Mode(RandomMode::Light))
    );
    assert_eq!(
        parse_selection("AUTO"),
        Some(Selection::Mode(RandomMode::Auto))
    );
    assert_eq!(
        parse_selection("random"),
        Some(Selection::Mode(RandomMode::Dark)),
        "back-compat alias"
    );
    assert_eq!(parse_selection("nope"), None);
}

#[test]
fn random_pick_pools_are_pure() {
    for current in ALL_THEMES {
        for seed in [0u64, 1, 42, 600_000, u64::MAX] {
            assert!(random_pick(current, seed, true).is_dark());
            assert!(!random_pick(current, seed, false).is_dark());
            assert_ne!(random_pick(current, seed, true), current);
            assert_ne!(random_pick(current, seed, false), current);
        }
    }
}

#[test]
fn apply_selection_modes_pick_from_their_pool_immediately() {
    let _g = guard();
    apply_selection(Selection::Mode(RandomMode::Light), 1_000);
    assert_eq!(mode(), Some(RandomMode::Light));
    assert!(is_random());
    assert!(
        !current_id().is_dark(),
        "light mode must land on a light theme"
    );
    apply_selection(Selection::Mode(RandomMode::Dark), 2_000);
    assert!(current_id().is_dark());
    apply_selection(Selection::Fixed(ThemeId::PaperDark), 3_000);
    assert_eq!(mode(), None);
    assert!(!is_random());
    assert_eq!(current_id(), ThemeId::PaperDark);
}

#[test]
fn auto_mode_follows_the_os_appearance() {
    let _g = guard();
    set_os_dark(true);
    apply_selection(Selection::Mode(RandomMode::Auto), 1_000);
    assert!(current_id().is_dark(), "auto + OS dark → dark pool");
    // OS flips to light: the NEXT tick (or re-apply) must land light.
    set_os_dark(false);
    ROTATED_MS.store(0, Ordering::Relaxed);
    assert!(tick_random(ROTATE_MS));
    assert!(!current_id().is_dark(), "auto + OS light → light pool");
    set_os_dark(true);
    apply_selection(Selection::Fixed(ThemeId::PaperDark), 2_000);
}

#[test]
fn tick_random_rotates_within_the_light_pool() {
    let _g = guard();
    apply_selection(Selection::Mode(RandomMode::Light), 0);
    for i in 1..=4u64 {
        ROTATED_MS.store(0, Ordering::Relaxed);
        assert!(tick_random(i * ROTATE_MS));
        assert!(!current_id().is_dark(), "tick {i} left the light pool");
    }
    apply_selection(Selection::Fixed(ThemeId::PaperDark), 0);
}

#[test]
fn selection_label_names_mode_or_theme() {
    let _g = guard();
    apply_selection(Selection::Fixed(ThemeId::Graphite), 0);
    assert_eq!(selection_label(), "graphite");
    apply_selection(Selection::Mode(RandomMode::Auto), 0);
    assert_eq!(selection_label(), "auto");
    apply_selection(Selection::Fixed(ThemeId::PaperDark), 0);
}

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
fn only_the_crt_and_modern_presets_carry_the_crt_style() {
    // The renderer keys the bloom post-process off `theme().crt`: the `crt-*`
    // family turns the full tube on, and the modern family rides the same
    // chain for its clean glow (retro knobs zeroed — see
    // `modern_glow_is_clean_of_retro_knobs`). Paper stays flat.
    for id in ALL_THEMES {
        let wants_bloom = id.as_str().starts_with("crt-") || id.theme().modern.is_some();
        assert_eq!(
            id.theme().crt.is_some(),
            wants_bloom,
            "{} crt style presence should be {wants_bloom}",
            id.as_str()
        );
    }
}

#[test]
fn the_five_phosphors_have_distinct_personalities() {
    // The whole point of `CrtStyle` over four global constants: no two
    // phosphors may share identical tunings (a done-criterion of the
    // holographic overhaul goal). The modern palettes carry a CrtStyle too
    // (bloom-only), so they join the uniqueness sweep: 5 tubes + 4 modern.
    let styles: Vec<(&str, CrtStyle)> = ALL_THEMES
        .iter()
        .filter_map(|id| id.theme().crt.map(|s| (id.as_str(), s)))
        .collect();
    assert_eq!(styles.len(), 9);
    for (i, (an, a)) in styles.iter().enumerate() {
        for (bn, b) in &styles[i + 1..] {
            assert_ne!(a, b, "{an} and {bn} share an identical CrtStyle");
        }
    }
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
    assert_eq!(ThemeId::Cobalt.next(), ThemeId::PaperDark); // last wraps to first
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
            let picked = random_pick(current, seed, RandomMode::Dark);
            // A pick from a pool the current theme isn't in still holds the
            // never-empty / determinism contract; it just may equal nothing
            // special. Only assert inequality when current is in the pool.
            if RandomMode::Dark.in_pool(current) {
                assert_ne!(picked, current, "seed {seed} picked the current theme");
            }
            // Same seed -> same pick (determinism).
            assert_eq!(random_pick(current, seed, RandomMode::Dark), picked);
        }
    }
    // Varying the seed actually varies the pick (not a constant function).
    let current = ThemeId::PaperDark;
    let picks: Vec<ThemeId> = (0u64..20)
        .map(|s| random_pick(current, s, RandomMode::Dark))
        .collect();
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
fn cycle_next_walks_the_five_modes_and_wraps() {
    let _g = guard();
    // From a pinned palette, the first step enters the dark rotation...
    apply_selection(Selection::Fixed(ThemeId::PaperDark), 0);
    assert_eq!(cycle_next(1), "dark");
    assert!(is_random());
    assert!(current_id().is_dark() && current_id().theme().crt.is_none());
    // ...then light...
    assert_eq!(cycle_next(2), "light");
    assert!(!current_id().is_dark());
    // ...then crt...
    assert_eq!(cycle_next(3), "crt");
    assert!(current_id().theme().crt.is_some() && current_id().theme().modern.is_none());
    // ...then modern...
    assert_eq!(cycle_next(4), "modern");
    assert!(current_id().theme().modern.is_some());
    // ...then auto, whose pool follows the reported OS appearance...
    set_os_dark(true);
    assert_eq!(cycle_next(5), "auto");
    assert!(current_id().is_dark() && current_id().theme().crt.is_none());
    // ...and wraps back to dark.
    assert_eq!(cycle_next(6), "dark");
    assert!(current_id().is_dark() && current_id().theme().crt.is_none());
    apply_selection(Selection::Fixed(ThemeId::PaperDark), 0);
}

#[test]
fn auto_is_advertised_and_follows_the_os_appearance() {
    let _g = guard();
    // Auto is a first-class listed theme (last, after the fixed pools).
    assert_eq!(THEME_MODES[THEME_MODES.len() - 1], RandomMode::Auto);
    // Its pool tracks set_os_dark: light OS → light paper palettes only.
    set_os_dark(false);
    assert!(ALL_THEMES
        .into_iter()
        .filter(|id| RandomMode::Auto.in_pool(*id))
        .all(|id| !id.theme().dark && id.theme().crt.is_none()));
    set_os_dark(true);
    assert!(ALL_THEMES
        .into_iter()
        .filter(|id| RandomMode::Auto.in_pool(*id))
        .all(|id| id.theme().dark && id.theme().crt.is_none()));
    apply_selection(Selection::Fixed(ThemeId::PaperDark), 0);
}

#[test]
fn auto_pools_pair_each_appearance_with_its_configured_side() {
    let _g = guard();
    // Dark side paired to the CRT pool: night is phosphor now.
    set_auto_pools(Some(Selection::Mode(RandomMode::Crt)), None);
    set_os_dark(true);
    apply_selection(Selection::Mode(RandomMode::Auto), 7);
    assert!(
        current_id().theme().crt.is_some(),
        "dark side must serve the CRT pool, got {:?}",
        current_id()
    );
    // The unpaired light side keeps its built-in light paper pool.
    set_os_dark(false);
    apply_selection(Selection::Mode(RandomMode::Auto), 8);
    assert!(!current_id().is_dark() && current_id().theme().crt.is_none());
    // A pinned side is a one-palette pool: always exactly that palette.
    set_auto_pools(Some(Selection::Fixed(ThemeId::MossBlotter)), None);
    set_os_dark(true);
    apply_selection(Selection::Mode(RandomMode::Auto), 9);
    assert_eq!(current_id(), ThemeId::MossBlotter);
    // ...and a rotation tick can't drift off a pinned side.
    assert_eq!(
        random_pick(current_id(), 12345, RandomMode::Auto),
        ThemeId::MossBlotter
    );
    // `auto` as its own side is dropped: default pool, no recursion.
    set_auto_pools(Some(Selection::Mode(RandomMode::Auto)), None);
    apply_selection(Selection::Mode(RandomMode::Auto), 10);
    assert!(current_id().is_dark() && current_id().theme().crt.is_none());
    // Reset shared state for the other tests.
    set_auto_pools(None, None);
    set_os_dark(true);
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
    assert_eq!(ThemeId::from_u8(13), ThemeId::MossBlotter);
    assert_eq!(ThemeId::from_u8(14), ThemeId::GlacierBond);
    assert_eq!(ThemeId::from_u8(15), ThemeId::CrtPaperwhite);
    assert_eq!(ThemeId::from_u8(16), ThemeId::Aurora);
    assert_eq!(ThemeId::from_u8(17), ThemeId::Nebula);
    assert_eq!(ThemeId::from_u8(18), ThemeId::Graphene);
    assert_eq!(ThemeId::from_u8(19), ThemeId::Cobalt);
    set_theme(ThemeId::PaperDark);
}

/// Mirrors `crew_app::anim::lerp_rgb`'s per-channel rounding exactly (crew-theme
/// has no dependency on crew-app, so this is inlined rather than shared) —
/// used below to reproduce `chatink::code_bg()`'s "page nudged 8% toward ink"
/// tint without importing crew-app.
fn lerp_rgb(a: (u8, u8, u8), b: (u8, u8, u8), t: f32) -> (u8, u8, u8) {
    let mix = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    (mix(a.0, b.0), mix(a.1, b.1), mix(a.2, b.2))
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

        // The chat markdown palette (crew-app `chatink`) draws code from
        // ansi[6] and structural markers (list bullets, quote bars) from
        // ansi[3]. Code text is never drawn on the bare page — it always sits
        // on `code_bg()`, the page lerped 8% toward ink (see
        // `chatink::code_bg`) — so ansi[6] is measured against that tint here,
        // not against page_bg. Markers (bullets, quote bars) really are drawn
        // with no background, so ansi[3] stays measured against page_bg. A new
        // preset with a washed-out cyan or yellow breaks chat rendering, and
        // this is where that gets caught. Measured worst cases when written:
        // ansi[6] 4.95 vs the code card (SEPIA_LIGHT), ansi[3] 4.64 vs
        // page_bg (IVORY_LEDGER).
        let code_bg = lerp_rgb(bg, t.ink, 0.08);
        assert!(
            cr(t.ansi[6], code_bg) >= 4.5,
            "{name}: ansi[6] (chat code) vs code card {:?} = {:.3} (need >= 4.5)",
            code_bg,
            cr(t.ansi[6], code_bg)
        );
        assert!(
            cr(t.ansi[3], bg) >= 4.5,
            "{name}: ansi[3] (chat marker) vs page_bg = {:.3} (need >= 4.5)",
            cr(t.ansi[3], bg)
        );

        // Diff line inks: chat's ```diff fences (`chatink::token_fg`
        // Added/Removed) and the viewer's .patch rung (`viewpane` diff_lines)
        // both draw the RAW ansi[2] (added) and ansi[1] (removed) slots, with
        // no `separated` walk — in chat on the code card, in the viewer on
        // the bare page. ansi[6] (hunk header) is already floored above.
        // Tripwire, not a behavior change: without it these slots answer only
        // to the 3.0 terminal floor above, so a future preset could slide
        // diff lines unreadable silently. Two surfaces, two floors:
        // - page_bg ≥ 4.5, matching the ansi[6]/ansi[3] chat-ink floors
        //   (worst case when written: graphite ansi[1] at 5.57);
        // - the chat code card ≥ 3.2. The card tint is `chatink::CODE_BG_MIX`
        //   (0.18 — deeper than the 0.08 the assertions above predate), and
        //   against THAT surface today's presets bottom out at 3.39 (graphite
        //   ansi[1]), so 4.5 is not honestly available. 3.2 still trips a
        //   slide to the terminal floor, and on the card hue plus the +/−
        //   marker carry the signal, not brightness.
        let diff_bg = lerp_rgb(bg, t.ink, 0.18);
        for (slot, what) in [(1usize, "diff removed"), (2usize, "diff added")] {
            assert!(
                cr(t.ansi[slot], diff_bg) >= 3.2,
                "{name}: ansi[{slot}] ({what}) vs code card {:?} = {:.3} (need >= 3.2)",
                diff_bg,
                cr(t.ansi[slot], diff_bg)
            );
            assert!(
                cr(t.ansi[slot], bg) >= 4.5,
                "{name}: ansi[{slot}] ({what}) vs page_bg = {:.3} (need >= 4.5)",
                cr(t.ansi[slot], bg)
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
    // absolute term carries it (see paperbg.wgsl). The MODERN family is the
    // deliberate exception: its pages are glass, not newsprint — zero grain.
    for id in ALL_THEMES {
        let t = id.theme();
        if t.modern.is_some() {
            assert_eq!(t.grain, 0.0, "{}: modern pages carry no grain", id.as_str());
        } else {
            assert_eq!(t.grain, 1.2, "{}: grain", id.as_str());
        }
    }
}

/// The modern family's page carries the dot lattice INSTEAD of grain: a
/// deliberate identity swap (glass + dots vs newsprint speckle). Strength
/// stays a whisper — a mix weight past ~0.5 would read as wallpaper.
#[test]
fn modern_pages_carry_the_dot_lattice() {
    for id in ALL_THEMES {
        if let Some(m) = id.theme().modern {
            assert!(
                m.dots > 0.0 && m.dots <= 0.5,
                "{}: dot lattice in the whisper band, got {}",
                id.as_str(),
                m.dots
            );
        }
    }
}

#[test]
fn modern_glow_is_clean_of_retro_knobs() {
    // The modern family rides the CRT bloom chain for its halo, but it must
    // never look like a tube: curvature, scanlines and the bezel vignette
    // stay exactly zero, and the gradient poles must be distinct (a ring
    // with equal poles is just a flat stroke).
    for id in ALL_THEMES {
        let t = id.theme();
        let Some(m) = t.modern else { continue };
        let c = t.crt.expect("modern themes carry a bloom-only CrtStyle");
        assert_eq!(c.curvature, 0.0, "{}: curvature", id.as_str());
        assert_eq!(c.scanline, 0.0, "{}: scanline", id.as_str());
        assert_eq!(c.corner, 0.0, "{}: corner", id.as_str());
        assert!(c.glow > 0.0, "{}: a modern theme without glow", id.as_str());
        assert_ne!(m.pole_a, m.pole_b, "{}: gradient poles equal", id.as_str());
        assert!(m.drift_ms > 0, "{}: drift period", id.as_str());
        assert!(t.dark, "{}: modern palettes are dark", id.as_str());
    }
    // And the family is big enough to rotate: random_pick's contract needs
    // every pool to hold at least 4 palettes.
    let n = ALL_THEMES
        .iter()
        .filter(|id| id.theme().modern.is_some())
        .count();
    assert!(n >= 4, "modern pool has only {n} palettes");
}

#[test]
fn dark_paper_pages_lean_warm() {
    // Dark non-CRT pages read as warm charcoal/kraft: R strictly above B.
    for id in ALL_THEMES {
        let t = id.theme();
        if t.dark && t.crt.is_none() {
            assert!(
                t.page_bg.0 > t.page_bg.2,
                "{}: page_bg {:?} not warm (R must exceed B)",
                id.as_str(),
                t.page_bg
            );
        }
    }
}

#[test]
fn crt_pages_are_deep_cool_black() {
    // Neon retune: CRT tubes sit on a darker, cooler near-black so the
    // phosphor halo pops — max page channel ≤ 8, and never warm (R ≤ B+2).
    // Modern palettes carry a bloom-only CrtStyle but are NOT tubes — their
    // pages sit brighter and take any cast — so they are exempt.
    for id in ALL_THEMES {
        let t = id.theme();
        if t.crt.is_some() && t.modern.is_none() {
            let (r, g, b) = t.page_bg;
            assert!(
                r.max(g).max(b) <= 8,
                "{}: page_bg {:?} too bright for a neon tube",
                id.as_str(),
                t.page_bg
            );
            assert!(
                r <= b.saturating_add(2),
                "{}: page_bg {:?} warm — CRT pages stay cool",
                id.as_str(),
                t.page_bg
            );
        }
    }
}

#[test]
fn parse_selection_names_modes_and_alias() {
    // The three canonical names.
    assert_eq!(
        parse_selection("dark"),
        Some(Selection::Mode(RandomMode::Dark))
    );
    assert_eq!(
        parse_selection(" Light "),
        Some(Selection::Mode(RandomMode::Light))
    );
    assert_eq!(
        parse_selection("CRT"),
        Some(Selection::Mode(RandomMode::Crt))
    );
    assert_eq!(
        parse_selection("Modern"),
        Some(Selection::Mode(RandomMode::Modern))
    );
    // A pinned palette name still resolves (back-compat).
    assert_eq!(
        parse_selection("paper-light"),
        Some(Selection::Fixed(ThemeId::PaperLight))
    );
    // Pre-consolidation mode names still parse.
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
            // Dark pool: dark, non-CRT. Light pool: light, non-CRT. CRT pool:
            // the phosphor palettes. Each pick lands in the right pool.
            let d = random_pick(current, seed, RandomMode::Dark);
            assert!(
                d.is_dark() && d.theme().crt.is_none(),
                "dark pool: {}",
                d.as_str()
            );
            let l = random_pick(current, seed, RandomMode::Light);
            assert!(
                !l.is_dark() && l.theme().crt.is_none(),
                "light pool: {}",
                l.as_str()
            );
            let c = random_pick(current, seed, RandomMode::Crt);
            assert!(
                c.theme().crt.is_some() && c.theme().modern.is_none(),
                "crt pool: {}",
                c.as_str()
            );
            let m = random_pick(current, seed, RandomMode::Modern);
            assert!(m.theme().modern.is_some(), "modern pool: {}", m.as_str());
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

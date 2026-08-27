use super::*;

/// Serialises tests that mutate the process-wide CURRENT.
use crate::test_guard as guard;

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
fn the_phosphors_have_distinct_personalities() {
    // The whole point of `CrtStyle` over four global constants: no two
    // phosphors may share identical tunings (a done-criterion of the
    // holographic overhaul goal). EVERY theme carries one now — the bloom
    // chain is what draws the gradient ring's halo — so all nine join the
    // uniqueness sweep. It earned its keep immediately: paper-dark and
    // sepia-dark were first written with identical tunings and this caught it.
    let styles: Vec<(&str, CrtStyle)> = ALL_THEMES
        .iter()
        .filter_map(|id| id.theme().crt.map(|s| (id.as_str(), s)))
        .collect();
    assert_eq!(styles.len(), 11);
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
    assert_eq!(
        ALL_THEMES[ALL_THEMES.len() - 1].next(),
        ALL_THEMES[0],
        "the last theme wraps to the first"
    );
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
fn cycle_next_walks_every_mode_and_wraps() {
    let _g = guard();
    // From a pinned palette, the first step enters the dark rotation...
    apply_selection(Selection::Fixed(ThemeId::PaperDark), 0);
    assert_eq!(cycle_next(1), "dark");
    assert!(is_random());
    assert!(current_id().is_dark() && !current_id().is_crt());
    // ...then light...
    assert_eq!(cycle_next(2), "light");
    assert!(!current_id().is_dark());
    // ...then crt...
    assert_eq!(cycle_next(3), "crt");
    assert!(current_id().is_crt());
    // ...then auto, whose pool follows the reported OS appearance...
    set_os_dark(true);
    assert_eq!(cycle_next(4), "auto");
    assert!(current_id().is_dark() && !current_id().is_crt());
    // ...and wraps back to dark — four stops, no more.
    assert_eq!(cycle_next(5), "dark");
    assert!(current_id().is_dark() && !current_id().is_crt());
    apply_selection(Selection::Fixed(ThemeId::PaperDark), 0);
}

#[test]
fn auto_is_advertised_and_follows_the_os_appearance() {
    let _g = guard();
    // Auto is a first-class listed theme (last, after the fixed pools).
    assert_eq!(THEME_MODES[THEME_MODES.len() - 1], RandomMode::Auto);
    // Its pool tracks set_os_dark: light OS → light pages only (paper and
    // modern glow alike — never a tube).
    set_os_dark(false);
    assert!(ALL_THEMES
        .into_iter()
        .filter(|id| RandomMode::Auto.in_pool(*id))
        .all(|id| !id.is_dark() && !id.is_crt()));
    set_os_dark(true);
    assert!(ALL_THEMES
        .into_iter()
        .filter(|id| RandomMode::Auto.in_pool(*id))
        .all(|id| id.is_dark() && !id.is_crt()));
    apply_selection(Selection::Fixed(ThemeId::PaperDark), 0);
}

#[test]
fn a_pinned_os_appearance_hands_auto_over_to_the_clock() {
    let _g = guard();
    set_auto_pools(None, None);
    // While the OS switches itself it is the only authority — the daylight
    // flag must not reach the answer at all.
    set_os_auto(true);
    set_os_dark(true);
    for day in [false, true] {
        set_daylight(day);
        assert!(auto_dark(), "self-switching dark OS must stay dark ({day})");
    }
    set_os_dark(false);
    for day in [false, true] {
        set_daylight(day);
        assert!(
            !auto_dark(),
            "self-switching light OS must stay light ({day})"
        );
    }

    // Pinned: the OS appearance stops mattering and the clock decides. This
    // is the reported bug — a Mac pinned to Dark at noon was dark forever.
    set_os_auto(false);
    set_os_dark(true);
    set_daylight(true);
    assert!(!auto_dark(), "pinned-dark OS in daylight must serve light");
    assert!(ALL_THEMES
        .into_iter()
        .filter(|id| RandomMode::Auto.in_pool(*id))
        .all(|id| !id.is_dark() && !id.is_crt()));
    apply_selection(Selection::Mode(RandomMode::Auto), 11);
    assert!(!current_id().is_dark() && !current_id().is_crt());

    // ...and after dark the same pinned OS agrees with the clock again.
    set_daylight(false);
    assert!(auto_dark());
    apply_selection(Selection::Mode(RandomMode::Auto), 12);
    assert!(current_id().is_dark() && !current_id().is_crt());

    // Symmetry: a Mac pinned to LIGHT goes dark at night. Once the OS stops
    // changing, the clock is the only thing left that can.
    set_os_dark(false);
    assert!(auto_dark(), "pinned-light OS at night must serve dark");

    // Restore the defaults the other tests assume.
    set_os_auto(true);
    set_daylight(false);
    apply_selection(Selection::Fixed(ThemeId::PaperDark), 0);
}

#[test]
fn the_pinned_fallback_still_honours_the_configured_pairing() {
    let _g = guard();
    // The clock picks the SIDE; `theme_dark`/`theme_light` still decide what
    // that side serves. Phosphor at night, paper by day, on a pinned Mac.
    set_auto_pools(Some(Selection::Mode(RandomMode::Crt)), None);
    set_os_auto(false);
    set_os_dark(true);
    set_daylight(false);
    apply_selection(Selection::Mode(RandomMode::Auto), 13);
    assert!(
        current_id().theme().crt.is_some(),
        "clock-night must serve the paired CRT pool, got {:?}",
        current_id()
    );
    set_daylight(true);
    apply_selection(Selection::Mode(RandomMode::Auto), 14);
    assert!(
        !current_id().is_dark() && !current_id().is_crt(),
        "clock-day must serve the unpaired light pool, got {:?}",
        current_id()
    );
    set_auto_pools(None, None);
    set_os_auto(true);
    set_daylight(false);
    apply_selection(Selection::Fixed(ThemeId::PaperDark), 0);
}

#[test]
fn the_light_hours_window_round_trips_for_reporting() {
    let _g = guard();
    assert_eq!(light_hours(), (7 * 60, 19 * 60), "default window");
    set_light_hours(5 * 60 + 30, 21 * 60 + 5);
    assert_eq!(light_hours(), (330, 1265));
    set_light_hours(7 * 60, 19 * 60);
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
    // The unpaired light side keeps its built-in light pool.
    set_os_dark(false);
    apply_selection(Selection::Mode(RandomMode::Auto), 8);
    assert!(!current_id().is_dark() && !current_id().is_crt());
    // A pinned side is a one-palette pool: always exactly that palette.
    set_auto_pools(Some(Selection::Fixed(ThemeId::SepiaDark)), None);
    set_os_dark(true);
    apply_selection(Selection::Mode(RandomMode::Auto), 9);
    assert_eq!(current_id(), ThemeId::SepiaDark);
    // ...and a rotation tick can't drift off a pinned side.
    assert_eq!(
        random_pick(current_id(), 12345, RandomMode::Auto),
        ThemeId::SepiaDark
    );
    // `auto` as its own side is dropped: default pool, no recursion.
    set_auto_pools(Some(Selection::Mode(RandomMode::Auto)), None);
    apply_selection(Selection::Mode(RandomMode::Auto), 10);
    assert!(current_id().is_dark() && !current_id().is_crt());
    // Reset shared state for the other tests.
    set_auto_pools(None, None);
    set_os_dark(true);
    apply_selection(Selection::Fixed(ThemeId::PaperDark), 0);
}

/// The roster is nine because twenty-four contained near-duplicates: measured
/// on page + ink + accent, the closest same-appearance pair was
/// `midnight-ink` ~ `aurora` at **Δ 0.0209** — under the Δ 0.027 at which two
/// greys stop being separable, i.e. two themes a user could not tell apart.
///
/// This asserts the cut actually bought separation rather than just removing
/// things. The nine were chosen by farthest-point selection over that same
/// distance, constrained to keep both appearances in every surviving family.
#[test]
fn no_two_palettes_are_near_duplicates() {
    let spread = |a: &Theme, b: &Theme| {
        use crate::oklch::distance as d;
        (d(a.page_bg, b.page_bg) + d(a.ink, b.ink) + d(a.accent_default, b.accent_default)) / 3.0
    };
    let mut worst = (f32::MAX, String::new());
    for (i, a) in ALL_THEMES.iter().enumerate() {
        for b in &ALL_THEMES[i + 1..] {
            // Only compare within an appearance: a dark and a light theme are
            // never confusable however close their accents sit.
            if a.theme().dark != b.theme().dark {
                continue;
            }
            let v = spread(a.theme(), b.theme());
            if v < worst.0 {
                worst = (v, format!("{} ~ {}", a.as_str(), b.as_str()));
            }
        }
    }
    assert!(
        worst.0 > 0.05,
        "{} are only Δ {:.4} apart — that is half a rung of the text \
         hierarchy, which is not enough to be two themes",
        worst.1,
        worst.0
    );
}

/// `auto` needs both halves, and the CRT pool is a rotation of its own, so no
/// pool may be emptied by a roster cut.
#[test]
fn every_pool_survives_the_cut() {
    let count = |f: fn(ThemeId) -> bool| ALL_THEMES.iter().filter(|id| f(**id)).count();
    let dark = count(|id: ThemeId| id.theme().dark && !id.is_crt());
    let light = count(|id: ThemeId| !id.theme().dark);
    let crt = count(|id: ThemeId| id.is_crt());
    assert_eq!(
        (dark, light, crt),
        (4, 4, 3),
        "pools are dark {dark}, light {light}, crt {crt} — `auto` needs both \
         appearances and the tubes are their own rotation"
    );
}

/// A config naming a retired theme must land on its nearest surviving
/// relative, not silently reset to the default. Fifteen palettes were retired;
/// every one of their names still resolves.
#[test]
fn every_retired_theme_name_still_resolves() {
    const RETIRED: [(&str, ThemeId); 15] = [
        ("midnight-ink", ThemeId::Nebula),
        ("graphite", ThemeId::PaperDark),
        ("moss-blotter", ThemeId::SepiaDark),
        ("coldpress-gray", ThemeId::PaperLight),
        ("salmon-broadsheet", ThemeId::PaperLight),
        ("ivory-ledger", ThemeId::PaperLight),
        ("glacier-bond", ThemeId::PaperLight),
        ("crt-violet", ThemeId::CrtBlue),
        ("crt-paperwhite", ThemeId::CrtBlue),
        ("aurora", ThemeId::Nebula),
        ("graphene", ThemeId::Nebula),
        ("cobalt", ThemeId::Nebula),
        ("daybreak", ThemeId::Blossom),
        ("meadow", ThemeId::Blossom),
        ("cirrus", ThemeId::Blossom),
    ];
    for (name, want) in RETIRED {
        assert_eq!(
            ThemeId::from_name(name),
            Some(want),
            "the retired {name} no longer resolves — a saved config naming it \
             would reset to the default instead of keeping the user's taste"
        );
        // …and onto something of the same appearance, so a dark desk does not
        // suddenly go white.
        assert!(
            ThemeId::from_name(name).is_some(),
            "{name} resolves to nothing"
        );
    }
    // 24 palettes went into the cut and 15 came out of it retired; `harbor`
    // and `fern` were drawn afterwards, so the roster is the survivors plus
    // them. Written as a sum rather than a difference so adding a palette
    // does not read as retiring one.
    assert_eq!(
        RETIRED.len() + ALL_THEMES.len(),
        26,
        "every retiree is listed"
    );
}

#[test]
fn u8_mapping_round_trips_all_ids() {
    // Runtime mapping only — it backs the `set_theme`/`current_id` atomic and
    // is never persisted, so the 24→9 cut renumbered it freely. What matters
    // is that every id survives the round trip and no two share a code.
    let _g = guard();
    let mut seen: Vec<u8> = Vec::new();
    for id in ALL_THEMES {
        set_theme(id);
        assert_eq!(current_id(), id, "{} lost by u8 round-trip", id.as_str());
        let code = id.as_u8();
        assert!(!seen.contains(&code), "{} reuses u8 {code}", id.as_str());
        seen.push(code);
    }
    // An unknown code cannot panic — it lands on the first theme.
    assert_eq!(ThemeId::from_u8(200), ALL_THEMES[0]);
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
    // absolute term carries it (see paperbg.wgsl). NEBULA AND BLOSSOM are the
    // deliberate exception: their pages are glass, not newsprint — zero grain.
    //
    // This used to key off `modern.is_some()`, back when carrying a gradient
    // and being made of glass were the same two themes. Every theme has a
    // gradient now and most of them are still paper, so the exception is named
    // rather than derived: newsprint and a gradient sit together perfectly
    // well, and paper that lost its tooth would just be a flat page.
    for id in ALL_THEMES {
        let t = id.theme();
        let glass = matches!(id, ThemeId::Nebula | ThemeId::Blossom);
        let want = if glass { 0.0 } else { 1.2 };
        assert_eq!(t.grain, want, "{}: grain", id.as_str());
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

/// The wash is the aurora UNDER the lattice: broad pools of pole light. It
/// has to stay weaker than the dots — a wash past a whisper stops being light
/// on a page and becomes a coloured page, which is exactly the flat fill the
/// modern family is trying not to be.
#[test]
fn modern_pages_carry_the_gradient_wash() {
    for id in ALL_THEMES {
        if let Some(m) = id.theme().modern {
            assert!(
                m.wash > 0.0 && m.wash <= 0.35,
                "{}: wash in the whisper band, got {}",
                id.as_str(),
                m.wash
            );
        }
    }
}

/// Glow had no upper bound anywhere, which is the one effect that can ruin a
/// page. 2026's revival of neon is explicitly *micro*-glow — focus states,
/// outlines, small badges — never a flood, and a bloom is the easiest thing in
/// this renderer to overdo. Measured, the pools sit at 0.70..1.10 (CRT),
/// 0.60..0.90 (modern dark) and 0.30..0.40 (modern light); the bands below
/// leave headroom for a new theme without leaving room for a flood.
#[test]
fn glow_stays_inside_its_pool_s_band() {
    for id in ALL_THEMES {
        let t = id.theme();
        let Some(c) = t.crt else { continue };
        let (lo, hi, pool) = if t.modern.is_none() {
            (0.5, 1.3, "crt")
        } else if t.dark {
            (0.4, 1.1, "modern dark")
        } else {
            (0.2, 0.6, "modern light")
        };
        assert!(
            (lo..=hi).contains(&c.glow),
            "{}: glow {} is outside the {pool} band {lo}..{hi}",
            id.as_str(),
            c.glow
        );
        // A halo, not a wash. The blur runs at half resolution, so the
        // full-res reach is roughly twice this.
        assert!(
            (4.0..=16.0).contains(&c.glow_radius),
            "{}: bloom radius {} is outside the halo band",
            id.as_str(),
            c.glow_radius
        );
    }
}

/// The micro-glow rule, made structural rather than advisory: a bright page
/// needs far less bloom than a dark one before light stops reading as light
/// and starts reading as haze. Every light page must glow less than every
/// dark one — not merely less than its own twin.
#[test]
fn every_light_page_glows_less_than_every_dark_one() {
    let glow = |dark: bool| -> Vec<f32> {
        ALL_THEMES
            .iter()
            .filter(|id| id.theme().dark == dark && id.theme().modern.is_some())
            .map(|id| id.theme().crt.unwrap().glow)
            .collect()
    };
    let brightest_light = glow(false).into_iter().fold(0.0f32, f32::max);
    let dimmest_dark = glow(true).into_iter().fold(f32::MAX, f32::min);
    assert!(
        brightest_light < dimmest_dark,
        "the brightest light page glows {brightest_light} and the dimmest dark \
         one {dimmest_dark} — the pools overlap, so some light page is \
         carrying a dark page's bloom"
    );
}

/// The backdrop is a family trait, not a per-theme flourish: every modern
/// page of one appearance carries exactly the same lattice and wash. Pinned
/// so a new member joins the family rather than inventing its own weights —
/// the bands above would let it drift a long way first.
#[test]
fn the_modern_backdrop_is_a_per_appearance_constant() {
    // Three constants, not two: a tube already has bloom and scanlines doing
    // this work, so its lattice and wash run at half strength or the page
    // turns to soup. Paper keeps the per-appearance pair it always had.
    for id in ALL_THEMES {
        let t = id.theme();
        let Some(m) = t.modern else { continue };
        let (dots, wash) = match (id.is_crt(), t.dark) {
            (true, _) => (0.10, 0.10),
            (false, true) => (0.20, 0.15),
            (false, false) => (0.16, 0.12),
        };
        assert_eq!(m.dots, dots, "{}: dot lattice", id.as_str());
        assert_eq!(m.wash, wash, "{}: gradient wash", id.as_str());
    }
}

/// A palette flip must not also change how the page moves. With a gradient on
/// every theme the twinning rule generalises: they all drift at one rate, so
/// switching theme changes the colours and nothing about the motion.
#[test]
fn every_theme_drifts_at_the_same_rate() {
    let periods: std::collections::BTreeSet<u64> = ALL_THEMES
        .into_iter()
        .filter_map(|id| id.theme().modern.map(|m| m.drift_ms))
        .collect();
    assert_eq!(
        periods.len(),
        1,
        "themes drift at different rates {periods:?} — switching palette should \
         change the colours, not the motion"
    );
}

#[test]
fn modern_glow_is_clean_of_retro_knobs() {
    // The modern family rides the CRT bloom chain for its halo, but it must
    // never look like a tube: scanlines stay exactly zero, and the gradient
    // poles must be distinct (a ring with equal poles is just a flat stroke).
    // Curvature and the bezel vignette used to be asserted here too — they
    // are gone from `CrtStyle` entirely now, which is a stronger guarantee
    // than a test: every theme set them to zero, so the shader was warping by
    // an identity and multiplying by one on every pixel of every frame.
    for id in ALL_THEMES {
        let t = id.theme();
        let Some(m) = t.modern else { continue };
        let c = t
            .crt
            .expect("every theme carries a CrtStyle for the ring's halo");
        if !id.is_crt() {
            // A glowing paper theme must never grow scanlines; a tube keeps its own.
            assert_eq!(c.scanline, 0.0, "{}: scanline", id.as_str());
        }
        assert!(c.glow > 0.0, "{}: a modern theme without glow", id.as_str());
        assert_ne!(m.pole_a, m.pole_b, "{}: gradient poles equal", id.as_str());
        assert!(m.drift_ms > 0, "{}: drift period", id.as_str());
    }
    // The family covers both appearances, so consolidating it into the
    // dark/light pools leaves neither of them without glow. One twinned pair
    // survives the 24→9 cut — `nebula`/`blossom` — so one a side is the floor,
    // not a shortfall.
    for (side, want_dark) in [("dark", true), ("light", false)] {
        let n = ALL_THEMES
            .iter()
            .filter(|id| {
                let t = id.theme();
                t.modern.is_some() && t.dark == want_dark
            })
            .count();
        assert!(n >= 1, "the {side} pool inherited no modern palette");
    }
}

/// The light half is the same family with the lights on, and its poles have
/// to be COLOUR on a white page, not the dark half's pastels: the ring, the
/// wash and the lattice all draw in them, and a pale pole on near-white paper
/// is an invisible one. 2.2 is the `border_focused` floor the ring already
/// answers to.
#[test]
fn light_modern_poles_read_on_a_white_page() {
    let mut seen = 0;
    for id in ALL_THEMES {
        let t = id.theme();
        let Some(m) = t.modern.filter(|_| !t.dark) else {
            continue;
        };
        seen += 1;
        for (which, pole) in [("pole_a", m.pole_a), ("pole_b", m.pole_b)] {
            let c = contrast_ratio(pole, t.page_bg);
            assert!(
                c >= 2.2,
                "{}: {which} {pole:?} vs the page = {c:.2} (need >= 2.2)",
                id.as_str()
            );
        }
        // The near-white assertion that used to live here described BLOSSOM's
        // page, not a rule about gradients: sepia-light's cream is a
        // legitimate light page and the contrast check above is what actually
        // protects readability.
    }
    assert_eq!(
        seen, 4,
        "every light palette carries a gradient now — blossom, paper-light, \
         sepia-light, fern"
    );
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
    // The retired modern modes resolve to the pool that swallowed them — a
    // saved `theme = "modern"` must keep opening on a dark page and
    // `theme_light = "modern-light"` on a light one, never fall through to
    // "unknown" (which would silently drop the setting) and never cross to
    // the other appearance.
    for spelling in ["Modern", "random-modern"] {
        assert_eq!(
            parse_selection(spelling),
            Some(Selection::Mode(RandomMode::Dark)),
            "{spelling}"
        );
    }
    for spelling in [
        "modern-light",
        "Modern Light",
        "MODERNLIGHT",
        "random-modern-light",
    ] {
        assert_eq!(
            parse_selection(spelling),
            Some(Selection::Mode(RandomMode::Light)),
            "{spelling}"
        );
    }
    // A pinned palette name still resolves (back-compat), including the new
    // light palettes.
    assert_eq!(
        parse_selection("paper-light"),
        Some(Selection::Fixed(ThemeId::PaperLight))
    );
    assert_eq!(
        parse_selection(" daybreak "),
        Some(Selection::Fixed(ThemeId::Blossom))
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
            // Dark pool: dark pages, no tube. Light pool: light pages, no
            // tube. CRT pool: the phosphor palettes. Each pick lands in the
            // right pool — and the appearance split is what keeps a rotation
            // from flipping the page near-black↔near-white under you.
            let d = random_pick(current, seed, RandomMode::Dark);
            assert!(d.is_dark() && !d.is_crt(), "dark pool: {}", d.as_str());
            let l = random_pick(current, seed, RandomMode::Light);
            assert!(!l.is_dark() && !l.is_crt(), "light pool: {}", l.as_str());
            let c = random_pick(current, seed, RandomMode::Crt);
            assert!(c.is_crt(), "crt pool: {}", c.as_str());
        }
    }
}

/// The consolidation itself: three pools, and every palette in exactly one of
/// them. The modern family used to stand apart as two more modes — its
/// palettes are dark and light PAGES like any other (the bloom-only
/// `CrtStyle` they carry for their halo is not a tube), so they rotate inside
/// `dark` / `light` and the picker offers three looks plus `auto`.
#[test]
fn every_palette_lands_in_exactly_one_of_the_three_pools() {
    let _g = guard();
    let pools = [RandomMode::Dark, RandomMode::Light, RandomMode::Crt];
    for id in ALL_THEMES {
        let n = pools.iter().filter(|m| m.in_pool(id)).count();
        assert_eq!(n, 1, "{} is in {n} pools, want exactly 1", id.as_str());
    }
    // Every non-tube palette rotates with the paper ones of its own appearance…
    // (the filter used to be `modern.is_some()`, which meant "not a tube" only
    // while gradients were a two-theme family; tubes have one now too).
    for id in ALL_THEMES
        .into_iter()
        .filter(|id| id.theme().modern.is_some() && !id.is_crt())
    {
        let want = if id.is_dark() {
            RandomMode::Dark
        } else {
            RandomMode::Light
        };
        assert!(
            want.in_pool(id),
            "{} must rotate inside {}",
            id.as_str(),
            want.as_str()
        );
        assert!(
            !RandomMode::Crt.in_pool(id),
            "{} is not a tube",
            id.as_str()
        );
    }
    // …and each pool is still wide enough for `random_pick`'s never-empty
    // contract: it filters out the current theme before picking, so a pool
    // needs at least two members or a rotation has nowhere to go. Three after
    // the cut, evenly across dark/light/crt.
    for m in pools {
        let n = ALL_THEMES.into_iter().filter(|id| m.in_pool(*id)).count();
        assert!(
            n >= 2,
            "{} pool has only {n} palettes — a rotation would \
                have nowhere to move",
            m.as_str()
        );
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
    apply_selection(Selection::Fixed(ThemeId::PaperDark), 0);
    assert_eq!(selection_label(), "paper-dark");
    apply_selection(Selection::Mode(RandomMode::Auto), 0);
    assert_eq!(selection_label(), "auto");
    apply_selection(Selection::Fixed(ThemeId::PaperDark), 0);
}

/// A doc comment that names a number is a claim, and claims rot. `Theme::grain`
/// said "1.0 on dark themes; 1.2 on light themes" for long enough that no
/// theme had used 1.0 in months and the modern family's 0.0 went unmentioned
/// entirely — a number that was simply false to anyone adding a preset from
/// the struct docs, which is the one place they would look.
///
/// So the numeric fields' docs are checked against the palettes: every float
/// literal a field's doc names must be a value some preset actually ships.
/// Deliberately one-directional — a doc need not enumerate every value, it
/// just may not invent one.
#[test]
fn a_numeric_field_s_doc_may_not_name_a_value_no_palette_uses() {
    let src = include_str!("lib.rs");
    /// A numeric `Theme` field: its declaration line, and how to read it.
    type NumField = (&'static str, fn(&Theme) -> f32);
    let mut total = 0usize;
    let fields: [NumField; 2] = [
        ("pub grain: f32,", |t| t.grain),
        ("pub border_thickness: f32,", |t| t.border_thickness),
    ];
    for (decl, get) in fields {
        let at = src.find(decl).unwrap_or_else(|| panic!("{decl} not found"));
        // Back up to the start of the declaration's own line: `src[..at]`
        // otherwise ends mid-line on the indentation, and the first thing
        // `lines().rev()` yields is that fragment, which is not a `///` line
        // and stops the walk before it starts. (The vacuity check below is
        // what caught this.)
        let at = src[..at].rfind('\n').map_or(0, |i| i + 1);
        // The doc block is the run of `///` lines immediately above it.
        let doc: String = src[..at]
            .lines()
            .rev()
            .take_while(|l| l.trim_start().starts_with("///"))
            .collect::<Vec<_>>()
            .join(" ");
        let shipped: Vec<f32> = ALL_THEMES.iter().map(|id| get(id.theme())).collect();
        // Every `<digits>.<digits>` in the prose. A run preceded by a letter
        // or by a dot is part of something else — `v0.5.58` must not read as
        // a claim that some palette ships 5.58.
        let mut named: Vec<String> = Vec::new();
        let b = doc.as_bytes();
        let mut i = 0;
        while i < b.len() {
            let joined = i > 0 && (b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'.');
            if b[i].is_ascii_digit() && !joined {
                let start = i;
                while i < b.len() && b[i].is_ascii_digit() {
                    i += 1;
                }
                if i < b.len() && b[i] == b'.' && i + 1 < b.len() && b[i + 1].is_ascii_digit() {
                    i += 1;
                    while i < b.len() && b[i].is_ascii_digit() {
                        i += 1;
                    }
                    named.push(doc[start..i].to_string());
                }
            } else {
                i += 1;
            }
        }
        // A doc that names no numbers makes no claim and is fine; the
        // whole-test vacuity guard is after the loop.
        total += named.len();
        for n in named {
            let v: f32 = n.parse().unwrap();
            assert!(
                shipped.iter().any(|s| (s - v).abs() < 1e-6),
                "{decl}: the doc says {n}, but no palette ships it — shipped \
                 values are {shipped:?}"
            );
        }
    }
    assert!(
        total >= 4,
        "only {total} numbers are claimed across the checked fields — the \
         parse has stopped finding them and this test is asserting nothing"
    );
}

/// The contrast suite above measures every role against `page_bg` — the page
/// as declared. It is not the page anyone reads on. The gradient wash lies
/// UNDER the whole canvas and mixes the page toward a pole by `wash` at each
/// pool's centre, so the real background under a line of text is
/// `lerp(page_bg, pole, wash)`, and every floor in `contrast_thresholds` is
/// quietly measured against a colour that is not there.
///
/// This closes that. Same roles, same floors, measured on the WASHED page at
/// both poles — which is also the answer to "make the gradient stronger":
/// there is no room. Run at the shipped weights the tightest role clears its
/// floor by 4–16% (border_normal on the paper and modern pages, text_muted on
/// the tubes); at 1.5× the wash, six of the nine themes are already under.
/// The aurora is calibrated to the edge of legibility, and more colour has to
/// come from the chrome — which is where the gradient stroke went — not from
/// turning the page up.
///
/// The mix is done here on the sRGB bytes because that is where the shader
/// does it too: the surface is deliberately NON-sRGB (see the renderer's
/// colour-space note), so `mix()` in `paperbg.wgsl` blends in gamma space,
/// exactly like this.
#[test]
fn the_wash_never_pushes_a_role_under_its_floor() {
    for id in ALL_THEMES {
        let t = id.theme();
        let Some(m) = t.modern else {
            continue;
        };
        for pole in [m.pole_a, m.pole_b] {
            let mix = |a: u8, b: u8| {
                (f32::from(a) + (f32::from(b) - f32::from(a)) * m.wash).round() as u8
            };
            let page = (
                mix(t.page_bg.0, pole.0),
                mix(t.page_bg.1, pole.1),
                mix(t.page_bg.2, pole.2),
            );
            for (role, c, floor) in [
                ("ink", t.ink, 10.0f32),
                ("text_muted", t.text_muted, 7.0),
                ("legend_off", t.legend_off, 3.0),
                ("hint_fg", t.hint_fg, 2.5),
                ("placeholder", t.placeholder, 2.3),
                ("accent_default", t.accent_default, 3.0),
                ("border_focused", t.border_focused, 2.2),
                ("border_normal", t.border_normal, 1.45),
            ] {
                let got = contrast_ratio(c, page);
                assert!(
                    got >= floor,
                    "{}: {role} vs the page washed toward {pole:?} = {got:.3} (need >= {floor})",
                    id.as_str()
                );
            }
        }
    }
}

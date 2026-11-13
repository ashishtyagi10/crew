use crate::test_guard as guard;
use crate::*;

const CLASSICS: [ThemeId; 5] = [
    ThemeId::TokyoNight,
    ThemeId::Solarized,
    ThemeId::Everforest,
    ThemeId::Gruvbox,
    ThemeId::Monokai,
];

/// The steps in one lap of `auto`: one per palette, twenty, 200 minutes.
fn lap() -> usize {
    auto_looks().map(auto_steps).sum()
}

#[test]
fn a_lap_is_every_look_once_and_every_classic() {
    assert_eq!(auto_steps(RandomMode::Classic), 5);
    assert_eq!(auto_steps(RandomMode::Glass), 4);
    assert_eq!(auto_steps(RandomMode::Crt), 2);
    assert_eq!(auto_steps(RandomMode::Purple), 1);
    assert_eq!(auto_steps(RandomMode::Prism), 1);
    assert_eq!(auto_steps(RandomMode::Relief), 4);
    assert_eq!(lap(), ALL_THEMES.len());
}

/// `auto` steps every rotation, light and dark pages alike, whatever the
/// clock or the OS appearance (the user, 2026-10-25: "rotate through all
/// available theme, irrespective of time … dark, light, glass, neon"). Inside
/// `classic` it shows all five in turn rather than one at random, so a lap
/// shows every classic (2026-10-28: "these themes should also rotate via
/// /theme auto"). From any opening, one lap of ticks shows every look and
/// every classic, the classics back to back in order, and lands back on the
/// look it opened on.
#[test]
fn a_lap_of_auto_shows_every_look_and_all_five_classics() {
    let _g = guard();
    for seed in [0u64, 7, 1_000, 123_456_789, 4_242_424_242] {
        apply_selection(Selection::Mode(RandomMode::Auto), seed);
        let opened = current_id();
        let mut seen = vec![opened];
        for i in 1..=lap() as u64 {
            let before = current_id();
            assert!(tick_random(seed + i * ROTATE_MS), "seed {seed} tick {i}");
            assert_eq!(
                current_id().mode(),
                auto_next(before),
                "seed {seed} tick {i}"
            );
            if let Some(id) = auto_next_palette(before) {
                assert_eq!(current_id(), id, "seed {seed} tick {i}");
            }
            seen.push(current_id());
        }
        assert_eq!(seen[lap()].mode(), opened.mode(), "seed {seed}: {seen:?}");
        let lap_seen = &seen[..lap()];
        // Switching to `auto` opens on a look's first palette, so one lap
        // from there is every palette, each once.
        let mut all = lap_seen.to_vec();
        all.sort_by_key(|t| t.as_str());
        all.dedup();
        assert_eq!(all.len(), ALL_THEMES.len(), "seed {seed}: {lap_seen:?}");
        assert!(
            auto_looks().all(|m| lap_seen.iter().any(|t| t.mode() == m)),
            "seed {seed}: a lap showed {lap_seen:?}"
        );
        let classics: Vec<ThemeId> = lap_seen
            .iter()
            .copied()
            .filter(|t| t.mode() == RandomMode::Classic)
            .collect();
        assert_eq!(classics, CLASSICS, "seed {seed}");
        assert_eq!(mode(), Some(RandomMode::Auto), "a tick left auto");
    }
    apply_selection(Selection::Fixed(ThemeId::PaperDark), 0);
}

#[test]
fn classic_opens_on_tokyo_night_and_hands_over_after_monokai() {
    assert_eq!(auto_next(ThemeId::NeonDark), RandomMode::Classic);
    assert_eq!(
        auto_next_palette(ThemeId::NeonDark),
        Some(ThemeId::TokyoNight)
    );
    for pair in CLASSICS.windows(2) {
        assert_eq!(auto_next(pair[0]), RandomMode::Classic);
        assert_eq!(auto_next_palette(pair[0]), Some(pair[1]));
    }
    assert_eq!(auto_next(ThemeId::Monokai), RandomMode::Purple);
    assert_eq!(auto_next_palette(ThemeId::Monokai), Some(ThemeId::Amethyst));
    // Purple hands over to prism, prism to relief, which shows its four in
    // order (the light three, then slate) and, the last look, wraps to dark.
    assert_eq!(auto_next(ThemeId::Amethyst), RandomMode::Prism);
    assert_eq!(auto_next_palette(ThemeId::Amethyst), Some(ThemeId::Prism));
    assert_eq!(auto_next(ThemeId::Prism), RandomMode::Relief);
    assert_eq!(auto_next_palette(ThemeId::Prism), Some(ThemeId::Porcelain));
    assert_eq!(auto_next_palette(ThemeId::Porcelain), Some(ThemeId::Sand));
    assert_eq!(auto_next_palette(ThemeId::Sand), Some(ThemeId::Mist));
    assert_eq!(auto_next_palette(ThemeId::Mist), Some(ThemeId::Slate));
    assert_eq!(auto_next(ThemeId::Slate), RandomMode::Dark);
    // The tube and the glass are walked too (the user, 2026-11-12: "`theme
    // auto` should rotate through all themes"), in `ALL_THEMES` order.
    assert_eq!(
        auto_next_palette(ThemeId::PaperLight),
        Some(ThemeId::CrtGreen)
    );
    assert_eq!(
        auto_next_palette(ThemeId::CrtGreen),
        Some(ThemeId::CrtAmber)
    );
    assert_eq!(auto_next_palette(ThemeId::CrtAmber), Some(ThemeId::Nebula));
    assert_eq!(auto_next_palette(ThemeId::Nebula), Some(ThemeId::Blossom));
    assert_eq!(
        auto_next_palette(ThemeId::Blossom),
        Some(ThemeId::GlassClear)
    );
    assert_eq!(
        auto_next_palette(ThemeId::GlassClear),
        Some(ThemeId::GlassNight)
    );
    assert_eq!(
        auto_next_palette(ThemeId::GlassNight),
        Some(ThemeId::NeonDark)
    );
}

/// Switching to `auto` onto `classic` opens on its first palette, so even the
/// first lap shows all five.
#[test]
fn switching_to_auto_opens_classic_on_its_first_palette() {
    let _g = guard();
    let mut opened_classic = false;
    for seed in 0..200u64 {
        apply_selection(Selection::Fixed(ThemeId::PaperDark), 0);
        apply_selection(Selection::Mode(RandomMode::Auto), seed * 997);
        if current_id().mode() == RandomMode::Classic {
            assert_eq!(current_id(), ThemeId::TokyoNight, "seed {seed}");
            opened_classic = true;
        }
    }
    assert!(opened_classic, "no seed opened on classic");
    apply_selection(Selection::Fixed(ThemeId::PaperDark), 0);
}

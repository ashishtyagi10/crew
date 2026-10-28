use crate::test_guard as guard;
use crate::*;

const CLASSICS: [ThemeId; 5] = [
    ThemeId::TokyoNight,
    ThemeId::Solarized,
    ThemeId::Everforest,
    ThemeId::Gruvbox,
    ThemeId::Monokai,
];

/// The steps in one lap of `auto`: one per look, plus a step for each
/// classic after the first — eleven, 110 minutes.
fn lap() -> usize {
    auto_looks().map(auto_steps).sum()
}

#[test]
fn a_lap_is_every_look_once_and_every_classic() {
    assert_eq!(auto_steps(RandomMode::Classic), 5);
    assert_eq!(auto_steps(RandomMode::Glass), 1);
    assert_eq!(auto_steps(RandomMode::Purple), 1);
    assert_eq!(lap(), 11);
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
    // The looks `auto` still picks from at random (crt, glass) have no next
    // palette set.
    assert_eq!(auto_next_palette(ThemeId::PaperLight), None);
    assert_eq!(auto_next_palette(ThemeId::CrtGreen), None);
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

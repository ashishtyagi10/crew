//! The upgrade heals, each proved to fire once and only on a value the user
//! never chose — the half of a heal that is easy to get wrong.
use super::*;

/// The pre-gamma default did two jobs; `/gamma` took one back, so a config
/// still carrying that default overshoots. The heal moves it — and nothing chosen.
#[test]
fn the_smoothing_heal_moves_the_old_default_and_nothing_else() {
    let mut cfg = CrewConfig {
        font_smooth: crate::config::SMOOTH_BEFORE_GAMMA,
        ..CrewConfig::default()
    };
    assert!(cfg.adopt_rebalanced_smoothing());
    assert_eq!(cfg.font_smooth, crew_render::DEFAULT_SMOOTH);

    // A chosen strength survives, including the new default itself (the heal
    // is one-shot but must be idempotent if it ever runs twice).
    for chosen in [0u8, 42, 170, 255, crew_render::DEFAULT_SMOOTH] {
        let mut cfg = CrewConfig {
            font_smooth: chosen,
            ..CrewConfig::default()
        };
        assert!(!cfg.adopt_rebalanced_smoothing(), "moved a chosen {chosen}");
        assert_eq!(cfg.font_smooth, chosen);
    }
}

/// The 0.19.62 heal: a config on the 0.19.28 pair takes the undilated one; a chosen half pins both.
#[test]
fn upgrading_adopts_the_undilated_text_pair() {
    let mut cfg = CrewConfig {
        font_smooth: crate::config::SMOOTH_AFTER_GAMMA,
        font_gamma: crate::config::GAMMA_WITH_DILATION,
        ..Default::default()
    };
    assert!(cfg.adopt_undilated_text());
    assert_eq!(cfg.font_smooth, crew_render::DEFAULT_SMOOTH);
    assert_eq!(cfg.font_gamma, crew_render::DEFAULT_TEXT_GAMMA);
    assert!(!cfg.adopt_undilated_text(), "the heal is one-shot");

    for (smooth, gamma) in [
        (42, crate::config::GAMMA_WITH_DILATION),
        (crate::config::SMOOTH_AFTER_GAMMA, 42),
    ] {
        let mut cfg = CrewConfig {
            font_smooth: smooth,
            font_gamma: gamma,
            ..Default::default()
        };
        assert!(
            !cfg.adopt_undilated_text(),
            "moved a chosen pair ({smooth}, {gamma})"
        );
    }
}

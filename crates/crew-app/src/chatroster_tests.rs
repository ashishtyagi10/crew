use super::*;
use crew_theme::{contrast_ratio, ALL_THEMES};

const NAMES: [&str; 20] = [
    "planner", "coder", "scout", "smith", "reviewer", "tester", "writer", "user", "alpha", "beta",
    "gamma", "delta", "ops", "research", "critic", "builder", "fixer", "judge", "guard", "muse",
];

#[test]
fn agent_color_is_stable_and_distinguishes_names() {
    let _g = crate::app::theme_test_guard();
    assert_eq!(agent_color("planner"), agent_color("planner"));
    assert_ne!(agent_color("planner"), agent_color("coder"));
}

/// Every preset, twenty names: each reads on the page at the mark floor, and
/// the pool is wide enough that twenty agents get at least eight colours.
/// On main the pool was six bright ANSI slots, so `distinct >= 8` fails
/// with 6 on every preset.
#[test]
fn agent_colours_read_on_every_preset_and_spread_over_twenty_names() {
    let _g = crate::app::theme_test_guard();
    for id in ALL_THEMES {
        let t = id.theme();
        let colours: Vec<(u8, u8, u8)> = NAMES.iter().map(|n| agent_color_on(n, t)).collect();
        for (name, c) in NAMES.iter().zip(&colours) {
            let got = contrast_ratio(*c, t.page_bg);
            assert!(got >= 3.0, "{}: @{name} vs page = {got:.2}", id.as_str());
        }
        let mut distinct = colours.clone();
        distinct.sort();
        distinct.dedup();
        assert!(
            distinct.len() >= 8,
            "{}: {} distinct colours across 20 names",
            id.as_str(),
            distinct.len()
        );
    }
}

/// Reused, not copied: an agent's colour IS the `@project` tag colour of the
/// same name, slot hash and lift included, on every preset.
#[test]
fn an_agent_wears_the_tag_pools_colour() {
    let _g = crate::app::theme_test_guard();
    for id in ALL_THEMES {
        let t = id.theme();
        for name in NAMES {
            assert_eq!(agent_color_on(name, t), crew_theme::tag_color(name, t));
        }
    }
    assert_eq!(
        agent_color("Planner"),
        agent_color("planner"),
        "case-insensitive"
    );
}

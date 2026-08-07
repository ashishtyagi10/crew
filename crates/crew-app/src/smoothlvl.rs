//! Named font-smoothing strengths — the single ladder behind `/smooth`'s
//! keywords and the Settings form's Smoothing picker. Both surfaces read and
//! write the same `font_smooth` config key; sharing the table here is what
//! keeps them from ever disagreeing about what "medium" means.

/// The named steps, in picker order. `medium` is the renderer's calibrated
/// CoreText-style default.
pub(crate) const SMOOTH_LEVELS: [(&str, u8); 4] = [
    ("off", 0),
    ("light", 60),
    ("medium", crew_render::DEFAULT_SMOOTH),
    ("heavy", 170),
];

/// The strength behind a `/smooth` keyword, if it is one.
pub(crate) fn strength_of(name: &str) -> Option<u8> {
    SMOOTH_LEVELS
        .iter()
        .find(|&&(n, _)| n == name)
        .map(|&(_, s)| s)
}

/// Display label for a strength: the keyword when it sits on the ladder, the
/// raw number otherwise (a custom `/smooth 42` shows as `42`, not a lie).
pub(crate) fn label_of(strength: u8) -> String {
    SMOOTH_LEVELS
        .iter()
        .find(|&&(_, s)| s == strength)
        .map(|&(n, _)| n.to_string())
        .unwrap_or_else(|| strength.to_string())
}

/// Step to the next/previous named level, wrapping. A custom strength joins
/// the ladder from the top (same idiom as the theme picker's unknown case).
pub(crate) fn cycle(strength: u8, back: bool) -> u8 {
    let n = SMOOTH_LEVELS.len();
    let cur = SMOOTH_LEVELS
        .iter()
        .position(|&(_, s)| s == strength)
        .unwrap_or(0);
    let next = if back {
        (cur + n - 1) % n
    } else {
        (cur + 1) % n
    };
    SMOOTH_LEVELS[next].1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keywords_map_to_their_strengths() {
        assert_eq!(strength_of("off"), Some(0));
        assert_eq!(strength_of("light"), Some(60));
        assert_eq!(strength_of("medium"), Some(crew_render::DEFAULT_SMOOTH));
        assert_eq!(strength_of("heavy"), Some(170));
        assert_eq!(strength_of("glassy"), None);
    }

    #[test]
    fn labels_name_the_ladder_and_number_the_rest() {
        assert_eq!(label_of(0), "off");
        assert_eq!(label_of(crew_render::DEFAULT_SMOOTH), "medium");
        assert_eq!(label_of(42), "42");
    }

    #[test]
    fn cycle_wraps_both_ways_and_adopts_custom_values() {
        assert_eq!(cycle(0, false), 60);
        assert_eq!(cycle(170, false), 0, "forward wraps heavy → off");
        assert_eq!(cycle(0, true), 170, "backward wraps off → heavy");
        assert_eq!(cycle(42, false), 60, "custom joins from the ladder top");
    }
}

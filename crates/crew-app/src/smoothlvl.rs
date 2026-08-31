//! Named font-smoothing strengths — the single ladder behind `/smooth`'s
//! keywords and the Settings form's Smoothing picker. Both surfaces read and
//! write the same `font_smooth` config key; sharing the table here is what
//! keeps them from ever disagreeing about what "medium" means.

/// The named steps, in picker order. `off` is the renderer's default now:
/// the coverage curve behind `/gamma` delivers the outline's own light on
/// its own, and the darkening on top of it only spread that light over 45%
/// more pixels (see [`crew_render::DEFAULT_SMOOTH`]). The rest of the ladder
/// is unchanged, for anyone who wants the fatter Terminal.app look back —
/// `medium` is still the strength that look was calibrated at.
pub(crate) const SMOOTH_LEVELS: [(&str, u8); 4] =
    [("off", 0), ("light", 40), ("medium", 70), ("heavy", 120)];

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
#[path = "smoothlvl_tests.rs"]
mod tests;

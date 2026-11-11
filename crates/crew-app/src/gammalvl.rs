//! Named text-gamma amounts — the single ladder behind `/gamma`'s keywords
//! and the Settings form's Text gamma picker. Both surfaces read and write
//! the same `font_gamma` config key; sharing the table here is what keeps
//! them from ever disagreeing about what "medium" means. Sibling of
//! [`crate::smoothlvl`], which does the same for `/smooth`.

/// The named steps, in picker order. `off` is the renderer's default now:
/// text weight comes from widening the outline (`/smooth`), which keeps a
/// glyph's soft edge, where this curve buys its weight by flattening it (see
/// [`crew_render::DEFAULT_TEXT_GAMMA`]). `full` is the whole sRGB correction,
/// for the physically exact rim.
pub(crate) const GAMMA_LEVELS: [(&str, u8); 4] =
    [("off", 0), ("light", 65), ("medium", 130), ("full", 255)];

/// The amount behind a `/gamma` keyword, if it is one.
pub(crate) fn amount_of(name: &str) -> Option<u8> {
    GAMMA_LEVELS
        .iter()
        .find(|&&(n, _)| n == name)
        .map(|&(_, a)| a)
}

/// Display label for an amount: the keyword when it sits on the ladder, the
/// raw number otherwise (a custom `/gamma 42` shows as `42`, not a lie).
pub(crate) fn label_of(amount: u8) -> String {
    GAMMA_LEVELS
        .iter()
        .find(|&&(_, a)| a == amount)
        .map(|&(n, _)| n.to_string())
        .unwrap_or_else(|| amount.to_string())
}

/// Step to the next/previous named level, wrapping. A custom amount joins
/// the ladder from the top, the same idiom as [`crate::smoothlvl::cycle`].
pub(crate) fn cycle(amount: u8, back: bool) -> u8 {
    let n = GAMMA_LEVELS.len();
    let cur = GAMMA_LEVELS
        .iter()
        .position(|&(_, a)| a == amount)
        .unwrap_or(0);
    let next = if back {
        (cur + n - 1) % n
    } else {
        (cur + 1) % n
    };
    GAMMA_LEVELS[next].1
}

#[cfg(test)]
#[path = "gammalvl_tests.rs"]
mod tests;

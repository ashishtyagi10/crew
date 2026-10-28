//! What `/theme auto` shows next.
//!
//! `auto` walks the looks in [`THEME_MODES`] order, a step every
//! [`ROTATE_MS`](crate::ROTATE_MS). Most looks get one step, on a palette
//! from their pool at random. A look in [`WALKED`] gets one step per palette,
//! in [`ALL_THEMES`] order, so every lap shows all of them: with one random
//! classic a lap, seeing all five took several laps (the user, 2026-10-28:
//! the classics "should also rotate via /theme auto").
use crate::{random_pick, spread, RandomMode, ThemeId, ALL_THEMES, THEME_MODES};

/// The looks `auto` shows palette by palette rather than one at random.
const WALKED: [RandomMode; 1] = [RandomMode::Classic];

/// The looks `auto` walks, in [`THEME_MODES`] order: every theme but
/// `auto` itself.
pub fn auto_looks() -> impl Iterator<Item = RandomMode> {
    THEME_MODES.into_iter().filter(|&m| m != RandomMode::Auto)
}

/// How many steps `auto` gives `look` a lap: one per palette for a look it
/// walks palette by palette, else one.
pub fn auto_steps(look: RandomMode) -> usize {
    if WALKED.contains(&look) {
        palettes(look).count()
    } else {
        1
    }
}

/// `look`'s palettes, in [`ALL_THEMES`] order.
fn palettes(look: RandomMode) -> impl Iterator<Item = ThemeId> {
    ALL_THEMES.into_iter().filter(move |t| t.mode() == look)
}

/// The palette after `current` inside its own look, when `auto` walks that
/// look palette by palette and `current` is not its last.
fn next_in_look(current: ThemeId) -> Option<ThemeId> {
    if !WALKED.contains(&current.mode()) {
        return None;
    }
    palettes(current.mode())
        .skip_while(|&t| t != current)
        .nth(1)
}

/// The palette `auto` opens `look` on when that is fixed: a walked look's
/// first, or a one-page look's page.
fn opening(look: RandomMode) -> Option<ThemeId> {
    if WALKED.contains(&look) {
        palettes(look).next()
    } else {
        look.sole_palette()
    }
}

/// The look `auto` turns to after `current`: its own while a walked look
/// still has palettes to show, else the next of [`auto_looks`], wrapping, so
/// every look comes round in turn. Neither the clock nor the OS appearance
/// has a say — `auto` used to serve the appearance's page, which on a Mac
/// pinned to Dark was `dark` for good.
pub fn auto_next(current: ThemeId) -> RandomMode {
    if next_in_look(current).is_some() {
        return current.mode();
    }
    let looks: Vec<RandomMode> = auto_looks().collect();
    let i = looks.iter().position(|&m| m == current.mode()).unwrap_or(0);
    looks[(i + 1) % looks.len()]
}

/// The palette `auto` shows after `current` when that is decided in
/// advance: the next of a walked look, or the opening of the next look.
/// `None` when the next look picks from its pool at random (`glass`, `crt`).
pub fn auto_next_palette(current: ThemeId) -> Option<ThemeId> {
    next_in_look(current).or_else(|| opening(auto_next(current)))
}

/// The palette `auto` turns to after `current`, picking from `seed` where
/// the next look has no set order.
pub(crate) fn auto_step(current: ThemeId, seed: u64) -> ThemeId {
    auto_next_palette(current).unwrap_or_else(|| random_pick(current, seed, auto_next(current)))
}

/// The palette switching TO `auto` opens on: in any look but `current`'s,
/// from `seed`. Not simply [`auto_step`], because at launch `current` is
/// always the zeroed default, and every launch would open on the same look.
pub(crate) fn auto_start(current: ThemeId, seed: u64) -> ThemeId {
    let others: Vec<RandomMode> = auto_looks().filter(|&m| m != current.mode()).collect();
    let look = others[spread(seed, others.len())];
    opening(look).unwrap_or_else(|| random_pick(current, seed, look))
}

#[cfg(test)]
#[path = "autowalk_tests.rs"]
mod tests;

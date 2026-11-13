//! What `/theme auto` shows next.
//!
//! `auto` walks EVERY palette, a step every [`ROTATE_MS`](crate::ROTATE_MS):
//! the looks in [`THEME_MODES`] order, and inside each look its palettes in
//! [`ALL_THEMES`] order, so one lap shows them all (the user, 2026-11-12:
//! "`theme auto` should rotate through all themes"). It used to give most
//! looks one step a lap, on a palette from their pool at random, and only
//! the classics and the neumorphic four a step each: seeing every glass or
//! tube palette took several laps, and luck.
use crate::{spread, RandomMode, ThemeId, ALL_THEMES, THEME_MODES};

/// The looks `auto` walks, in [`THEME_MODES`] order: every theme but
/// `auto` itself.
pub fn auto_looks() -> impl Iterator<Item = RandomMode> {
    THEME_MODES.into_iter().filter(|&m| m != RandomMode::Auto)
}

/// How many steps `auto` gives `look` a lap: one per palette.
pub fn auto_steps(look: RandomMode) -> usize {
    palettes(look).count()
}

/// `look`'s palettes, in [`ALL_THEMES`] order.
fn palettes(look: RandomMode) -> impl Iterator<Item = ThemeId> {
    ALL_THEMES.into_iter().filter(move |t| t.mode() == look)
}

/// The palette after `current` inside its own look, unless `current` is
/// its last.
fn next_in_look(current: ThemeId) -> Option<ThemeId> {
    palettes(current.mode())
        .skip_while(|&t| t != current)
        .nth(1)
}

/// The palette `auto` opens `look` on: its first.
fn opening(look: RandomMode) -> Option<ThemeId> {
    palettes(look).next()
}

/// The look `auto` turns to after `current`: its own while that look
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

/// The palette `auto` shows after `current`: the next of its look, or the
/// opening of the next look. `None` only for a look with no palette.
pub fn auto_next_palette(current: ThemeId) -> Option<ThemeId> {
    next_in_look(current).or_else(|| opening(auto_next(current)))
}

/// The palette `auto` turns to after `current`.
pub(crate) fn auto_step(current: ThemeId) -> ThemeId {
    auto_next_palette(current).unwrap_or(current)
}

/// The palette switching TO `auto` opens on: in any look but `current`'s,
/// from `seed`. Not simply [`auto_step`], because at launch `current` is
/// always the zeroed default, and every launch would open on the same look.
pub(crate) fn auto_start(current: ThemeId, seed: u64) -> ThemeId {
    let others: Vec<RandomMode> = auto_looks().filter(|&m| m != current.mode()).collect();
    let look = others[spread(seed, others.len())];
    opening(look).unwrap_or(current)
}

#[cfg(test)]
#[path = "autowalk_tests.rs"]
mod tests;

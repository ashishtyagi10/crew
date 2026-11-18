//! The metal plates' light moves on its own.
//!
//! A brushed plate under a light that never moves is a picture of metal (the
//! user, 2026-11-17: "not shiny enough and no animation"). So on a metal
//! theme the sheen streak drifts slowly back and forth across the window, as
//! a plate does when you sway in front of it, and every [`GLINT_EVERY_MS`] a
//! narrow glint runs across it, burning along the bevels as it passes.
//!
//! Unbounded, unlike the glides in `wants_animation_frame`: it is ambient
//! motion, like an idle crew pane's breathing dot, and paced the same way —
//! its own branch in `poll_panes`, only while the window has focus, and
//! never at `Motion = off`, where the light rests where the pointer put it.
use crate::motion::MotionLevel;
use std::f32::consts::TAU;

/// One full sway of the sheen, there and back, ms. Slow: light that moved
/// faster than a person sways would read as a screensaver.
pub(crate) const DRIFT_MS: u64 = 16_000;
/// How far the sheen sways each way, as a fraction of the window's width.
pub(crate) const DRIFT: f32 = 0.22;
/// A glint starts this often, ms…
pub(crate) const GLINT_EVERY_MS: u64 = 9_000;
/// …and takes this long to cross the window.
pub(crate) const GLINT_RUN_MS: u64 = 1_400;

#[cfg(test)]
thread_local! {
    /// A test's fixed clock for the sheen (`None`: the light at rest), so a
    /// screenshot draws the same frame every run.
    pub(crate) static AT: std::cell::Cell<Option<u64>> = const { std::cell::Cell::new(None) };
}

/// The sweep the glass pass is handed at `now_ms` (crew-render `set_sweep`):
/// the sheen's drift, the glint's place across the window and its strength.
/// At rest (`[0, -1, 0, 0]`) on a theme that is not metal and at Motion off;
/// at Subtle the sway is half as wide and no glint runs.
pub(crate) fn sweep(now_ms: u64, metal: bool, motion: MotionLevel) -> [f32; 4] {
    if !metal || motion == MotionLevel::Off {
        return REST;
    }
    let full = motion == MotionLevel::Full;
    let phase = (now_ms % DRIFT_MS) as f32 / DRIFT_MS as f32;
    let drift = DRIFT * (TAU * phase).sin() * if full { 1.0 } else { 0.5 };
    let run = (now_ms % GLINT_EVERY_MS) as f32 / GLINT_RUN_MS as f32;
    if !full || run >= 1.0 {
        return [drift, -1.0, 0.0, 0.0];
    }
    // Eased across, from just off the left edge to just off the right, and
    // brightest mid-window so it never pops in or out.
    let eased = run * run * (3.0 - 2.0 * run);
    let at = -0.15 + 1.3 * eased;
    [drift, at, (std::f32::consts::PI * run).sin(), 0.0]
}

/// The sweep for this frame: the live clock, or under test [`AT`]'s (the
/// light at rest when none is set, so every other shot is unchanged).
pub(crate) fn live() -> [f32; 4] {
    #[cfg(not(test))]
    let at = Some((crate::anim::now_ms(), crate::motion::level()));
    #[cfg(test)]
    let at = AT
        .with(std::cell::Cell::get)
        .map(|t| (t, MotionLevel::Full));
    at.map_or(REST, |(now, motion)| sweep(now, metal_theme(), motion))
}

/// The light at rest: no drift, no glint.
pub(crate) const REST: [f32; 4] = [0.0, -1.0, 0.0, 0.0];

/// Whether the sheen owes frames: a metal theme, motion on, window focused.
pub(crate) fn moving(metal: bool, motion: MotionLevel, focused: bool) -> bool {
    metal && focused && motion != MotionLevel::Off
}

/// The live theme is metal.
pub(crate) fn metal_theme() -> bool {
    crew_theme::theme().metal() > 0.0
}

/// A shot's light: the pointer's tilt and the metal's, as `render` hands
/// them to the live grid.
#[cfg(test)]
pub(crate) fn light_shot(grid: &mut crew_render::CellGrid, tilt: (f32, f32)) {
    grid.set_glass_light(tilt);
    grid.set_metal_sweep(live());
}

#[cfg(test)]
#[path = "metalsheen_tests.rs"]
mod tests;

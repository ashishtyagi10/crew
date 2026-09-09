//! The crew pane's per-agent colour helper, shared by the chip grid,
//! waterfall, message cards, pane legends and the footer so every agent
//! reads the same colour everywhere it appears.
//!
//! The colour is `crew_theme`'s tag pool — the same twelve chromatic slots
//! and the same contrast lift a `@project` tag gets in `/todo` — rather than
//! a hash into the six bright ANSI slots this used to do. Six slots gave
//! seven agents a collision by the pigeonhole principle, and a raw bright
//! slot on a light page measured under 3.0:1: a name you could not read.
//! The pool is lifted to the mark floor on every preset and spread by
//! brightness on a tube, where the ANSI slots are one hue.

/// Stable colour for an agent name on the ACTIVE theme: `planner` renders
/// the same colour every frame and across panes, and agents are told apart
/// at a glance.
pub(crate) fn agent_color(name: &str) -> (u8, u8, u8) {
    agent_color_on(name, crew_theme::theme())
}

/// [`agent_color`] on an explicit theme, so the contract can sweep every
/// preset without touching the global.
pub(crate) fn agent_color_on(name: &str, t: &crew_theme::Theme) -> (u8, u8, u8) {
    crew_theme::tag_color(name, t)
}

#[cfg(test)]
#[path = "chatroster_tests.rs"]
mod tests;

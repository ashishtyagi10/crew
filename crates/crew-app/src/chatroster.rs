//! The crew pane's per-agent colour helper, shared by the chip grid,
//! waterfall, and message cards so every agent reads the same colour
//! everywhere it appears.

/// Stable colour for an agent name: a small hash picks from the theme's bright
/// ANSI palette (skipping black/white), so `planner` renders the same colour
/// every frame and across panes, and agents are told apart at a glance.
pub(crate) fn agent_color(name: &str) -> (u8, u8, u8) {
    // Bright red..bright cyan (ANSI 9..=14): distinct, readable on the page bg.
    let palette = &crew_theme::theme().ansi[9..=14];
    let h = name.bytes().fold(0xcbf2_9ce4u32, |h, b| {
        (h ^ b as u32).wrapping_mul(0x0100_0193)
    });
    palette[(h as usize) % palette.len()]
}

#[cfg(test)]
#[path = "chatroster_tests.rs"]
mod tests;

//! The footer's working-agent badges, lit by their tokens.
//!
//! Line 3 names who is working as a badge on each agent's roster colour
//! (`summaryroute`). With three agents in a parallel pack the row says WHO
//! is working but not who is talking — so each badge's block now pulses
//! toward the page ink for [`crate::shimmer::PULSE_MS`] after that agent's
//! last token burst (the same `token_pulse` stamp the header's name reads,
//! `chatliveness`), and an agent quiet for [`IDLE_MS`] wears a dimmer block,
//! mixed toward the page, so the eye finds the live one.
//!
//! The label's ink is not touched here: `segment::badge` walks it to the
//! text floor against whatever block it lands on, so a lit block and a dim
//! one both keep a readable name. The block itself is floored at the mark
//! floor — its caps draw it on the page.
use std::collections::HashMap;

use crate::motion::MotionLevel;
use crate::shimmer::Color;

/// No burst for this long and the agent is idle: its block dims.
pub(crate) const IDLE_MS: u64 = 2_000;
/// How far an idle block sinks toward the page.
pub(crate) const DIM: f32 = 0.35;

/// The pane's token-burst stamps, with the animation clock they are read on.
#[derive(Clone, Copy)]
pub(crate) struct Pulses<'a> {
    pub map: &'a HashMap<String, u64>,
    pub now: u64,
}

/// The block an agent's badge sits on at `now`: its `roster` colour, lifted
/// toward `ink` by [`crate::shimmer::pulse_mix`] right after a burst at
/// `last`, sunk toward `page` by [`DIM`] once [`IDLE_MS`] have passed with
/// none, and the plain roster colour between — and for an agent that has
/// never burst (it is not idle, it has not started). Off never lifts; the
/// idle dim is a state, not a motion, and stands at every level.
pub(crate) fn block(
    roster: Color,
    ink: Color,
    page: Color,
    last: Option<u64>,
    now: u64,
    level: MotionLevel,
) -> Color {
    let floor = crew_theme::readable::MARK_FLOOR;
    let mix = crate::shimmer::pulse_mix(last, now, level);
    if mix > 0.0 {
        let want = crate::anim::lerp_rgb(roster, ink, mix);
        return crew_theme::readable::against(want, page, floor);
    }
    match last {
        Some(t) if now.saturating_sub(t) >= IDLE_MS => {
            let want = crate::anim::lerp_rgb(roster, page, DIM);
            crew_theme::readable::against(want, page, floor)
        }
        _ => roster,
    }
}

/// The block for `name`'s badge given the pane's `pulses` (`None` when the
/// footer is rendered without a pane — the tests' pure form — which is the
/// roster colour, as before).
pub(crate) fn agent_block(pulses: Option<Pulses<'_>>, name: &str) -> Color {
    let roster = crate::chatroster::agent_color(name);
    let Some(p) = pulses else {
        return roster;
    };
    let th = crew_theme::theme();
    block(
        roster,
        th.ink,
        th.page_bg,
        p.map.get(name).copied(),
        p.now,
        crate::motion::level(),
    )
}

#[cfg(test)]
#[path = "summarypulse_tests.rs"]
mod tests;

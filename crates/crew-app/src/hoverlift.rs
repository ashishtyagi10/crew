//! The card under the pointer rises a little on its glass.
//!
//! Focus lifts a card all the way (`spotlight::lift_for`); the pointer
//! resting on one lifts it a fraction of that — enough to say "this is the
//! one you would be clicking", never enough to be mistaken for focus. Moving
//! between cards hands the lift over the way focus does: the new card rises
//! as the old one settles, each on its own glide, so a pointer sweeping
//! across the grid leaves a soft wake rather than a row of flickers.
//!
//! Bounded like every other glide: each lift converges and snaps, and a card
//! that has settled back to the page is dropped, so `wants_animation_frame`
//! goes quiet once the pointer rests.
use crate::motion::MotionLevel;
use crate::pane::Pane;
use crew_render::PaneScene;

/// How far a hovered card rises, as a share of the focused lift.
pub(crate) const HOVER: f32 = 0.4;

/// Glide time constant, ms — about the focus clock's pace.
const TAU_MS: f32 = 90.0;

/// Under this a lift snaps to where it is heading.
const SNAP: f32 = 0.004;

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct HoverLift {
    /// `(pane index, 0..=1)` for every card still off the page.
    lifts: Vec<(usize, f32)>,
    target: Option<usize>,
}

impl HoverLift {
    /// Point at `pane` (or at nothing). Returns whether that changed — the
    /// caller's cue to ask for a frame.
    pub(crate) fn aim(&mut self, pane: Option<usize>) -> bool {
        let changed = self.target != pane;
        self.target = pane;
        if let Some(p) = pane.filter(|p| !self.lifts.iter().any(|(i, _)| i == p)) {
            self.lifts.push((p, 0.0));
        }
        changed
    }

    /// Whether any card is still rising or settling.
    pub(crate) fn moving(&self) -> bool {
        self.lifts.iter().any(|&(i, t)| t != self.goal(i))
    }

    fn goal(&self, pane: usize) -> f32 {
        if self.target == Some(pane) {
            1.0
        } else {
            0.0
        }
    }

    /// One frame of glide; cards back on the page are forgotten.
    pub(crate) fn step(&mut self, dt_ms: u64, motion: MotionLevel) {
        let k = match motion {
            MotionLevel::Off => 1.0,
            _ => 1.0 - (-(dt_ms as f32) / TAU_MS).exp(),
        };
        let target = self.target;
        for (i, t) in &mut self.lifts {
            let goal = if target == Some(*i) { 1.0 } else { 0.0 };
            let next = *t + (goal - *t) * k;
            *t = if (next - goal).abs() < SNAP {
                goal
            } else {
                next
            };
        }
        self.lifts.retain(|&(i, t)| t > 0.0 || target == Some(i));
    }

    /// How far pane `i` has risen, `0..=HOVER`.
    pub(crate) fn lift(&self, i: usize) -> f32 {
        self.lifts
            .iter()
            .find(|(p, _)| *p == i)
            .map_or(0.0, |(_, t)| HOVER * t)
    }

    /// Raise each hovered pane's CARD scene — the glass one spanning the
    /// pane's rect — by its hover lift, never below the lift it already has.
    pub(crate) fn apply(&self, scenes: &mut [PaneScene], panes: &[Pane]) {
        for &(i, _) in &self.lifts {
            let (Some(p), l) = (panes.get(i), self.lift(i)) else {
                continue;
            };
            let r = p.rect;
            let card = |s: &&mut PaneScene| {
                s.glass && !s.overlay && (s.x, s.y, s.w, s.h) == (r.x, r.y, r.w, r.h)
            };
            if let Some(s) = scenes.iter_mut().find(card) {
                s.lift = s.lift.max(l);
            }
        }
    }
}

#[cfg(test)]
#[path = "hoverlift_tests.rs"]
mod tests;

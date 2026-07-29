//! Ghost cards: the frame a pane leaves behind on its way out.
//!
//! A closing pane is gone from `panes` the instant it closes — it has to be,
//! since everything downstream (focus clamping, the grid LRU, the nav rows)
//! reads that vector and would otherwise operate on a pane the user has
//! dismissed. So the *animation* cannot live on the pane. It lives here: a
//! closed or minimized card records where it was and how it was labelled, and
//! that record outlives the pane just long enough to collapse.
//!
//! The record is deliberately inert — a rect, a title, a timestamp and a
//! direction. It holds no pane state, no process, no channel; a ghost that
//! could still *do* something would be a pane that refused to die.
use crate::ease::Timeline;
use crate::layout::Rect;

/// How long a card takes to collapse. A shade quicker than the assemble
/// (`paneview::ASSEMBLE_MS`): dismissal should feel decisive, and a slow exit
/// animation is the one users learn to resent.
const COLLAPSE_MS: u64 = 300;

/// Where a departing card collapses toward.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Exit {
    /// Closed: the frame retracts into its own corners, the reverse of the way
    /// it was drawn.
    Closed,
    /// Minimized: the frame retracts *and* travels left toward the nav, which
    /// is where the pane has actually gone.
    Minimized,
}

/// One departing card.
#[derive(Clone, Debug)]
pub(crate) struct Ghost {
    pub(crate) rect: Rect,
    pub(crate) title: String,
    pub(crate) exit: Exit,
    timeline: Timeline,
}

impl Ghost {
    pub(crate) fn new(rect: Rect, title: String, exit: Exit, now: u64) -> Self {
        Self {
            rect,
            title,
            exit,
            timeline: Timeline::start(now, COLLAPSE_MS, crate::motion::level()),
        }
    }

    /// Whether this ghost still has frames to draw. At `Motion = off` it is
    /// false immediately, so a dismissed pane simply vanishes and nothing is
    /// scheduled — the ghost is created and dropped in the same frame.
    pub(crate) fn live(&self, now: u64) -> bool {
        self.timeline.live(now)
    }

    /// Assemble progress for the frame, running backwards: 1.0 at the moment
    /// of dismissal down to 0.0 when it is gone.
    pub(crate) fn collapse_t(&self, now: u64) -> f32 {
        1.0 - self.timeline.eased(now, crate::ease::out_cubic)
    }

    /// The rect to draw at `now`. A minimized card drifts toward the nav on
    /// its way out; a closed one stays put and simply retracts.
    pub(crate) fn rect_at(&self, now: u64) -> Rect {
        match self.exit {
            Exit::Closed => self.rect,
            Exit::Minimized => {
                let t = self.timeline.eased(now, crate::ease::out_cubic);
                Rect {
                    x: self.rect.x - self.rect.x * t,
                    ..self.rect
                }
            }
        }
    }
}

/// Drop every ghost that has finished. Called once per frame, so a ghost's
/// entire lifetime is bounded by its own timeline — there is no path by which
/// one accumulates.
pub(crate) fn prune(ghosts: &mut Vec<Ghost>, now: u64) {
    ghosts.retain(|g| g.live(now));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::motion::MotionLevel;

    fn rect() -> Rect {
        Rect {
            x: 400.0,
            y: 50.0,
            w: 300.0,
            h: 200.0,
        }
    }

    fn ghost(exit: Exit, now: u64) -> Ghost {
        crate::motion::set_level(MotionLevel::Full);
        Ghost::new(rect(), "build".into(), exit, now)
    }

    #[test]
    fn collapse_runs_from_whole_to_nothing() {
        let g = ghost(Exit::Closed, 1_000);
        assert!((g.collapse_t(1_000) - 1.0).abs() < 1e-6, "starts whole");
        assert_eq!(g.collapse_t(1_000 + COLLAPSE_MS), 0.0, "ends gone");
        assert!(g.collapse_t(1_150) < 1.0);
    }

    #[test]
    fn a_closed_card_stays_where_it_was() {
        let g = ghost(Exit::Closed, 0);
        assert_eq!(g.rect_at(0).x, 400.0);
        assert_eq!(g.rect_at(150).x, 400.0);
    }

    /// Minimize means "it went into the nav", so the card has to travel that
    /// way — a card that retracted in place would say "closed" instead.
    #[test]
    fn a_minimized_card_travels_toward_the_nav() {
        let g = ghost(Exit::Minimized, 0);
        assert_eq!(g.rect_at(0).x, 400.0);
        let mid = g.rect_at(150).x;
        assert!(mid < 400.0 && mid > 0.0, "mid-flight x was {mid}");
        assert!(
            g.rect_at(COLLAPSE_MS).x.abs() < 1e-3,
            "should reach the nav"
        );
    }

    #[test]
    fn prune_drops_only_finished_ghosts() {
        let mut gs = vec![ghost(Exit::Closed, 0), ghost(Exit::Closed, 10_000)];
        prune(&mut gs, 1_000);
        assert_eq!(gs.len(), 1, "the settled ghost should be gone");
    }

    /// Reduce-motion: no collapse, no scheduled frames, no ghost.
    #[test]
    fn motion_off_leaves_nothing_behind() {
        crate::motion::set_level(MotionLevel::Off);
        let g = Ghost::new(rect(), "build".into(), Exit::Closed, 5_000);
        assert!(!g.live(5_000));
        let mut gs = vec![g];
        prune(&mut gs, 5_000);
        assert!(gs.is_empty(), "a dismissed pane must simply vanish at off");
        crate::motion::set_level(MotionLevel::Full);
    }
}

//! What a pane is DOING: whether it is busy, whether it wants another
//! animation frame, and the timeline that records how long assembling a frame
//! took.
//!
//! Split from [`crate::paneview`] for the line cap, along the line between
//! building the scene and asking the pane about itself.
use crate::pane::{Pane, PaneContent};

/// How long a card takes to draw itself in.
pub(crate) const ASSEMBLE_MS: u64 = 380;

/// Period of the busy scan's round trip down a working card and back.
pub(crate) const SCAN_MS: u64 = 2_600;

/// This pane's assemble timeline. Scaled by the Motion setting, which is read
/// here rather than threaded through every scene call — at `off` the timeline
/// is born settled and the card is simply drawn.
pub(crate) fn spawn_timeline(p: &Pane) -> crate::ease::Timeline {
    crate::ease::Timeline::start(p.born_ms, ASSEMBLE_MS, crate::motion::level())
}

/// Whether a pane is doing background work, so its border shows the
/// indeterminate progress sweep (swarm planning/running, agent chat awaiting).
pub(crate) fn pane_busy(p: &Pane) -> bool {
    match &p.content {
        PaneContent::Swarm(s) => s.is_busy(),
        PaneContent::Chat(c) => c.is_busy(),
        PaneContent::Far(f) => f.is_busy(),
        // A walk of a big tree is work the card should show it is doing.
        PaneContent::Disk(d) => d.is_scanning(),
        _ => false,
    }
}

/// Busy or briefly animating (a message card fading in): the redraw-scheduling
/// predicate for `poll` — wider than [`pane_busy`], which alone decides the
/// card's busy sweep so a fade never reads as "working".
pub(crate) fn pane_animating(p: &Pane) -> bool {
    pane_busy(p)
        || match &p.content {
            PaneContent::Chat(c) => c.is_fading(),
            _ => false,
        }
}

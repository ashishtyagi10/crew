//! Focus spotlight: the focused pane holds full ink while every other pane's
//! *content* washes gently toward the page, so the eye lands on the active
//! surface the way stage light lands on the speaker. The wash rides the same
//! 260ms focus travel as the border brackets — focus moves, and ink follows
//! it: the old pane dims as the new one brightens, in one motion.
//!
//! Only content cells wash; the frame already tells focus with its border
//! color, and the wash is mild (15% toward the page) so unfocused terminals
//! stay fully readable — this is emphasis, not an overlay. The spotlight
//! follows `app.focused` even while the input bar owns the keys: the bar acts
//! on that pane, so it stays lit.
use crew_render::CellView;

/// How far unfocused content leans toward the page. Mild by design: readable
/// always, unmistakable in a full grid.
pub(crate) const DIM: f32 = 0.15;

/// The wash strength for pane `i` this frame. `spot` is the spotlit pane,
/// `prev` the one the spotlight just left, `t` the eased focus travel
/// (1.0 at rest). The spotlit pane brightens as `t` rises, the previous one
/// dims by the same clock, everyone else rests at [`DIM`].
pub(crate) fn dim_for(i: usize, spot: usize, prev: usize, t: f32) -> f32 {
    // Focus mode deepens the resting wash (see `focusmode`); the choreography
    // is unchanged, so entering the mode leans the whole grid further back
    // without any pane's dim jumping out of step with the focus travel.
    let rest = crate::focusmode::dim();
    if i == spot {
        rest * (1.0 - t)
    } else if i == prev {
        rest * t
    } else {
        rest
    }
}

/// How far pane `i`'s card has lifted off the page this frame — the glass
/// sheet's elevation. The same choreography as [`dim_for`], on the same clock:
/// the spotlit pane rises as `t` goes to 1, the one it left settles back, and
/// everyone else rests on the page. Not derived from the dim: focus mode can
/// zero the resting wash, and a card must still rise when it does.
pub(crate) fn lift_for(i: usize, spot: usize, prev: usize, t: f32) -> f32 {
    if i == spot {
        t
    } else if i == prev {
        1.0 - t
    } else {
        0.0
    }
}

/// How far the input bar floats off the page. It is the one control every
/// session starts from, so it never lies flat: at rest it rides as high as a
/// focused card, and while you type in it it rises a half-step above every
/// pane — the composer is the thing in front. (It was a well once, pressed
/// into the glass; on a dark page an inner shadow on a near-black sheet drew
/// nothing, and the bar read as a flat outline.)
pub(crate) fn composer_lift(typing: bool) -> f32 {
    if typing {
        1.5
    } else {
        1.0
    }
}

/// Apply the wash: every cell's ink leans `dim` toward the page. Backgrounds
/// stay put — a selection or status band keeps its shape, only its text dims.
pub(crate) fn wash(cells: &mut [CellView], dim: f32) {
    if dim <= 0.0 {
        return;
    }
    let bg = crew_theme::theme().page_bg;
    for c in cells.iter_mut() {
        c.fg = crate::anim::lerp_rgb(c.fg, bg, dim);
    }
}

#[cfg(test)]
#[path = "spotlight_tests.rs"]
mod tests;

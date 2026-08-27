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
mod tests {
    use super::*;

    #[test]
    fn at_rest_only_the_spotlit_pane_holds_full_ink() {
        assert_eq!(dim_for(2, 2, 0, 1.0), 0.0);
        assert_eq!(dim_for(0, 2, 0, 1.0), DIM, "the pane focus left is dim");
        assert_eq!(dim_for(1, 2, 0, 1.0), DIM, "bystanders are dim");
    }

    #[test]
    fn focus_travel_crossfades_old_and_new() {
        // Mid-travel: the new pane is half-lit, the old half-dimmed, and the
        // two strengths mirror each other exactly.
        let up = dim_for(2, 2, 0, 0.5);
        let down = dim_for(0, 2, 0, 0.5);
        assert!((up - DIM * 0.5).abs() < 1e-6);
        assert!((up - down).abs() < 1e-6);
        // At the start of travel the roles are fully swapped.
        assert_eq!(dim_for(2, 2, 0, 0.0), DIM);
        assert_eq!(dim_for(0, 2, 0, 0.0), 0.0);
    }

    #[test]
    fn wash_moves_ink_toward_the_page_but_leaves_backgrounds() {
        let t = crew_theme::theme();
        let mut cells = vec![CellView {
            col: 0,
            row: 0,
            c: 'x',
            fg: t.ink,
            bg: (10, 20, 30),
            bold: false,
            italic: false,
            ..Default::default()
        }];
        let before = cells[0].fg;
        wash(&mut cells, DIM);
        assert_ne!(cells[0].fg, before, "ink must move");
        assert_eq!(cells[0].bg, (10, 20, 30), "backgrounds must not");
        assert_eq!(
            cells[0].fg,
            crate::anim::lerp_rgb(before, t.page_bg, DIM),
            "wash is exactly the documented lean"
        );
        // Zero dim is a strict no-op.
        let unwashed = cells[0].fg;
        wash(&mut cells, 0.0);
        assert_eq!(cells[0].fg, unwashed);
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Interior cell grid of a fieldset card `w`×`h` px with one border cell per
/// side: `floor(px/cell) − 2`, min 1 per axis. The single source of the
/// rect→cells convention, shared by PTY sizing (`relayout_one`), card drawing
/// (`push_card`), and border-button hit-testing (`min_btn_rect`) so they can
/// never disagree about where a cell sits.
pub fn card_inner_cells(w: f32, h: f32, cell_w: f32, cell_h: f32) -> (u16, u16) {
    let cols = ((w / cell_w).floor() as u16).saturating_sub(2).max(1);
    let rows = ((h / cell_h).floor() as u16).saturating_sub(2).max(1);
    (cols, rows)
}

/// Pack `n` tiles near-square into `w`x`h` offset by `(ox, oy)`. Outer edges
/// keep the full `gap`; interior edges take half each, so the seam between two
/// adjacent panes is one `gap` — tiles sit closer to each other than to the
/// window chrome.
pub fn pane_rects_at(n: usize, ox: f32, oy: f32, w: f32, h: f32, gap: f32) -> Vec<Rect> {
    if n == 0 {
        return Vec::new();
    }
    let cols = (n as f32).sqrt().ceil() as usize;
    let rows = n.div_ceil(cols);
    let tile_w = w / cols as f32;
    let tile_h = h / rows as f32;
    let half = gap / 2.0;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let c = i % cols;
        let r = i / cols;
        let left = if c == 0 { gap } else { half };
        let right = if c == cols - 1 { gap } else { half };
        let top = if r == 0 { gap } else { half };
        let bottom = if r == rows - 1 { gap } else { half };
        out.push(Rect {
            x: ox + c as f32 * tile_w + left,
            y: oy + r as f32 * tile_h + top,
            w: tile_w - left - right,
            h: tile_h - top - bottom,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32) {
        assert!((a - b).abs() < 0.5, "{a} != {b}");
    }

    #[test]
    fn one_pane_fills_minus_gap() {
        let r = pane_rects_at(1, 0.0, 0.0, 800.0, 600.0, 0.0);
        assert_eq!(r.len(), 1);
        approx(r[0].x, 0.0);
        approx(r[0].y, 0.0);
        approx(r[0].w, 800.0);
        approx(r[0].h, 600.0);
    }

    #[test]
    fn two_panes_side_by_side() {
        let r = pane_rects_at(2, 0.0, 0.0, 800.0, 600.0, 0.0);
        assert_eq!(r.len(), 2);
        approx(r[0].w, 400.0);
        approx(r[1].x, 400.0);
        approx(r[0].h, 600.0);
    }

    #[test]
    fn four_panes_two_by_two() {
        let r = pane_rects_at(4, 0.0, 0.0, 800.0, 600.0, 0.0);
        assert_eq!(r.len(), 4);
        approx(r[0].w, 400.0);
        approx(r[0].h, 300.0);
        approx(r[3].x, 400.0);
        approx(r[3].y, 300.0);
    }

    #[test]
    fn offset_shifts_origin() {
        let r = pane_rects_at(1, 50.0, 30.0, 800.0, 600.0, 0.0);
        approx(r[0].x, 50.0);
        approx(r[0].y, 30.0);
    }

    #[test]
    fn zero_panes_empty() {
        assert!(pane_rects_at(0, 0.0, 0.0, 800.0, 600.0, 4.0).is_empty());
    }

    #[test]
    fn interior_seam_is_one_gap_outer_margin_full() {
        let r = pane_rects_at(2, 0.0, 0.0, 800.0, 600.0, 8.0);
        // Outer margins keep the full gap…
        approx(r[0].x, 8.0);
        approx(r[1].x + r[1].w, 792.0);
        approx(r[0].y, 8.0);
        approx(r[0].h, 584.0);
        // …while the seam between the two panes is a single gap, not two.
        approx(r[1].x - (r[0].x + r[0].w), 8.0);
    }
}

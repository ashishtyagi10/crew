use ratatui::layout::Rect;

use super::geometry::grid_rects;
use super::state::GridLayout;

/// Height (rows) reserved for the minimized thumbnail strip when any tile is
/// minimized.
pub const MINIMIZED_STRIP_HEIGHT: u16 = 3;

/// Concrete placement of every tile for one frame.
#[derive(Debug, Clone, Default)]
pub struct GridRects {
    /// Full-size tiles: `(tile_id, rect)` in most-recently-active order.
    pub full: Vec<(usize, Rect)>,
    /// Minimized thumbnails: `(tile_id, rect)` left-to-right.
    pub minimized: Vec<(usize, Rect)>,
}

/// Place a `GridLayout` into `area`: grid the full tiles in the main region,
/// and — when there are minimized tiles — reserve a bottom strip and lay them
/// out evenly across it.
pub fn compute_grid_layout(area: Rect, layout: &GridLayout) -> GridRects {
    let full_ids = layout.full();
    let min_ids = layout.minimized();
    if full_ids.is_empty() && min_ids.is_empty() {
        return GridRects::default();
    }

    let (grid_area, strip_area) = if min_ids.is_empty() {
        (area, None)
    } else {
        let strip_h = MINIMIZED_STRIP_HEIGHT.min(area.height);
        let grid_h = area.height.saturating_sub(strip_h);
        let grid = Rect::new(area.x, area.y, area.width, grid_h);
        let strip = Rect::new(area.x, area.y + grid_h, area.width, strip_h);
        (grid, Some(strip))
    };

    let full: Vec<(usize, Rect)> = full_ids
        .iter()
        .copied()
        .zip(grid_rects(grid_area, full_ids.len()))
        .collect();

    let minimized = match strip_area {
        Some(strip) => min_ids
            .iter()
            .copied()
            .zip(grid_rects(strip, min_ids.len()))
            .collect(),
        None => Vec::new(),
    };

    GridRects { full, minimized }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::GridLayout;
    use ratatui::layout::Rect;

    fn area() -> Rect {
        Rect::new(0, 0, 120, 40)
    }

    #[test]
    fn compose_empty_layout_is_empty() {
        let g = GridLayout::new();
        let out = compute_grid_layout(area(), &g);
        assert!(out.full.is_empty());
        assert!(out.minimized.is_empty());
    }

    #[test]
    fn compose_no_minimized_uses_full_area() {
        let mut g = GridLayout::new();
        g.add(0);
        let out = compute_grid_layout(area(), &g);
        assert_eq!(out.full.len(), 1);
        assert!(out.minimized.is_empty());
        // With no strip reserved, the single tile fills the whole area height.
        assert_eq!(out.full[0].1.height, 40);
        assert_eq!(out.full[0].0, 0); // id preserved
    }

    #[test]
    fn compose_reserves_strip_when_minimized_present() {
        let mut g = GridLayout::new();
        for id in 0..7 {
            g.add(id);
        }
        let out = compute_grid_layout(area(), &g);
        assert_eq!(out.full.len(), 6);
        assert_eq!(out.minimized.len(), 1);
        // The full grid is pushed up to make room for the bottom strip.
        let max_full_bottom = out.full.iter().map(|(_, r)| r.y + r.height).max().unwrap();
        let strip_top = out.minimized[0].1.y;
        assert!(max_full_bottom <= strip_top, "full grid overlaps strip");
        // Strip sits at the bottom with the reserved height.
        assert_eq!(out.minimized[0].1.height, MINIMIZED_STRIP_HEIGHT);
        assert_eq!(out.minimized[0].1.y, 40 - MINIMIZED_STRIP_HEIGHT);
    }

    #[test]
    fn compose_full_ids_match_layout_order() {
        let mut g = GridLayout::new();
        g.add(0);
        g.add(1);
        g.add(2);
        let out = compute_grid_layout(area(), &g);
        let ids: Vec<usize> = out.full.iter().map(|(id, _)| *id).collect();
        assert_eq!(ids, vec![2, 1, 0]);
    }

    #[test]
    fn compose_tiny_area_with_minimized_no_panic() {
        let mut g = GridLayout::new();
        for id in 0..7 {
            g.add(id);
        }
        // Area shorter than the strip: must not panic; the strip is clamped to the
        // area height and the full tiles are starved to zero height.
        let out = compute_grid_layout(Rect::new(0, 0, 120, 2), &g);
        assert_eq!(out.full.len(), 6);
        assert_eq!(out.minimized.len(), 1);
        assert!(out.minimized[0].1.height <= 2);
        assert!(out.full.iter().all(|(_, r)| r.height == 0));
    }
}

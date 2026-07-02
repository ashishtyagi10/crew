use crate::grid::state::{GridLayout, MAX_FULL_TILES};
use crate::layout::{pane_rects_at, Rect};

/// Cell rows reserved for the minimized thumbnail strip when any pane is
/// minimized.
pub const MINIMIZED_STRIP_ROWS: f32 = 4.0;

/// Concrete placement of every tile for one frame.
#[derive(Debug, Clone, Default)]
pub struct GridRects {
    /// Full-size tiles: `(pane_index, rect)`, sorted by pane index (stable
    /// positions — the LRU decides membership, not display order).
    pub full: Vec<(usize, Rect)>,
    /// Minimized thumbnails: `(pane_index, rect)`, sorted by pane index,
    /// left-to-right.
    pub minimized: Vec<(usize, Rect)>,
}

/// Place a `GridLayout` into `content`: grid the full tiles in the main region
/// and — when there are minimized tiles — reserve a bottom strip and lay them
/// out evenly across one row. Both sets render in stable pane-index order, so
/// focusing a pane changes which tiles are full but never reorders them.
pub fn compose_grid(content: Rect, layout: &GridLayout, cell_h: f32, gap: f32) -> GridRects {
    let mut full_ids = layout.full().to_vec();
    let mut min_ids = layout.minimized().to_vec();
    if full_ids.is_empty() && min_ids.is_empty() {
        return GridRects::default();
    }
    debug_assert!(full_ids.len() <= MAX_FULL_TILES);
    // Stable display order regardless of LRU recency.
    full_ids.sort_unstable();
    min_ids.sort_unstable();

    let strip_h = if min_ids.is_empty() {
        0.0
    } else {
        (MINIMIZED_STRIP_ROWS * cell_h + 2.0 * gap).min(content.h)
    };
    let grid_h = (content.h - strip_h).max(0.0);

    let full_rects = pane_rects_at(full_ids.len(), content.x, content.y, content.w, grid_h, gap);
    let full = full_ids.into_iter().zip(full_rects).collect();

    let minimized = if min_ids.is_empty() {
        Vec::new()
    } else {
        let strip_y = content.y + grid_h;
        strip_row(&min_ids, content.x, strip_y, content.w, strip_h, gap)
    };

    GridRects { full, minimized }
}

/// Lay `ids` left-to-right as equal-width tiles across one strip row. Like the
/// main grid, interior seams take half the gap per side; the strip's outer
/// edges keep the full gap.
fn strip_row(ids: &[usize], x: f32, y: f32, w: f32, h: f32, gap: f32) -> Vec<(usize, Rect)> {
    let n = ids.len();
    let tile_w = w / n as f32;
    let half = gap / 2.0;
    ids.iter()
        .copied()
        .enumerate()
        .map(|(i, idx)| {
            let left = if i == 0 { gap } else { half };
            let right = if i == n - 1 { gap } else { half };
            (
                idx,
                Rect {
                    x: x + i as f32 * tile_w + left,
                    y: y + gap,
                    w: (tile_w - left - right).max(0.0),
                    h: h - 2.0 * gap,
                },
            )
        })
        .collect()
}

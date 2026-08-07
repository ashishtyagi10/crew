use crate::grid::state::{GridLayout, MAX_FULL_TILES};
use crate::layout::{pane_rects_at, Rect};

/// Cell rows reserved for the minimized thumbnail strip when any pane is
/// minimized.
pub const MINIMIZED_STRIP_ROWS: f32 = 4.0;

/// Narrowest strip thumbnail worth drawing, in cells (border included):
/// enough for the frame, a few legend glyphs with their ellipsis, and the
/// marker slot. Below this a card is an unreadable sliver (and under 4 cells
/// `titled_card` draws nothing at all while the rect stays clickable).
pub const MIN_STRIP_TILE_COLS: f32 = 14.0;

/// Concrete placement of every tile for one frame.
#[derive(Debug, Clone, Default)]
pub struct GridRects {
    /// Full-size tiles: `(pane_index, rect)`, sorted by pane index (stable
    /// positions — the LRU decides membership, not display order).
    pub full: Vec<(usize, Rect)>,
    /// Minimized thumbnails: `(pane_index, rect)`, sorted by pane index,
    /// left-to-right.
    pub minimized: Vec<(usize, Rect)>,
    /// When the strip can't hold every minimized pane at a readable width:
    /// `(hidden_count, rect)` for the trailing `+N` tile standing in for the
    /// rest. The hidden panes stay reachable from the sidebar PANES list.
    pub overflow: Option<(usize, Rect)>,
}

impl GridRects {
    /// Pane indices minimized by the LRU but given NO thumbnail — the panes
    /// the `+N` overflow tile stands in for. They are not visible anywhere in
    /// the content area, so the sidebar marks their rows restorable (`[+]`)
    /// like every other off-screen pane. `all_minimized` is the layout's full
    /// LRU tail ([`crate::grid::GridLayout::minimized`]).
    pub fn strip_hidden(&self, all_minimized: &[usize]) -> Vec<usize> {
        if self.overflow.is_none() {
            return Vec::new();
        }
        all_minimized
            .iter()
            .copied()
            .filter(|i| !self.minimized.iter().any(|&(m, _)| m == *i))
            .collect()
    }
}

/// Place a `GridLayout` into `content`: grid the full tiles in the main region
/// and — when there are minimized tiles — reserve a bottom strip and lay them
/// out evenly across one row. Both sets render in stable pane-index order, so
/// focusing a pane changes which tiles are full but never reorders them.
pub fn compose_grid(
    content: Rect,
    layout: &GridLayout,
    cell_w: f32,
    cell_h: f32,
    gap: f32,
) -> GridRects {
    let mut full_ids = layout.full().to_vec();
    let min_lru = layout.minimized();
    if full_ids.is_empty() && min_lru.is_empty() {
        return GridRects::default();
    }
    debug_assert!(full_ids.len() <= MAX_FULL_TILES);
    // Stable display order regardless of LRU recency.
    full_ids.sort_unstable();

    let strip_h = if min_lru.is_empty() {
        0.0
    } else {
        (MINIMIZED_STRIP_ROWS * cell_h + 2.0 * gap).min(content.h)
    };
    let grid_h = (content.h - strip_h).max(0.0);

    let full_rects = pane_rects_at(full_ids.len(), content.x, content.y, content.w, grid_h, gap);
    let full = full_ids.into_iter().zip(full_rects).collect();

    let (minimized, overflow) = if min_lru.is_empty() {
        (Vec::new(), None)
    } else {
        let strip_y = content.y + grid_h;
        // A crowded strip caps how many thumbnails it shows rather than
        // shrinking them into slivers. Membership follows the LRU (the
        // most-recently-active minimized panes stay visible), display order
        // stays sorted by pane index like everything else on the canvas.
        let cap = strip_cap(content.w, cell_w, gap);
        let (shown, hidden) = if min_lru.len() <= cap {
            (min_lru, 0)
        } else {
            // Reserve the last slot for the `+N` overflow tile.
            let keep = cap.saturating_sub(1);
            (&min_lru[..keep], min_lru.len() - keep)
        };
        let mut min_ids = shown.to_vec();
        min_ids.sort_unstable();
        let slots = min_ids.len() + usize::from(hidden > 0);
        let mut rects = strip_row(slots, content.x, strip_y, content.w, strip_h, gap);
        // The overflow tile owns the last slot (strip_row returned `slots`
        // rects, so the pop can't miss).
        let overflow = if hidden > 0 {
            rects.pop().map(|r| (hidden, r))
        } else {
            None
        };
        (min_ids.into_iter().zip(rects).collect(), overflow)
    };

    GridRects {
        full,
        minimized,
        overflow,
    }
}

/// How many strip tiles fit `w` px at [`MIN_STRIP_TILE_COLS`] each (plus a
/// gap of seam apiece). Always at least one.
fn strip_cap(w: f32, cell_w: f32, gap: f32) -> usize {
    let min_w = MIN_STRIP_TILE_COLS * cell_w + gap;
    if min_w <= 0.0 {
        return 1;
    }
    ((w / min_w) as usize).max(1)
}

/// Lay `n` equal-width tiles left-to-right across one strip row. Like the
/// main grid, interior seams take half the gap per side; the strip's outer
/// edges keep the full gap.
fn strip_row(n: usize, x: f32, y: f32, w: f32, h: f32, gap: f32) -> Vec<Rect> {
    let tile_w = w / n as f32;
    let half = gap / 2.0;
    (0..n)
        .map(|i| {
            let left = if i == 0 { gap } else { half };
            let right = if i == n - 1 { gap } else { half };
            Rect {
                x: x + i as f32 * tile_w + left,
                y: y + gap,
                w: (tile_w - left - right).max(0.0),
                h: h - 2.0 * gap,
            }
            .snapped()
        })
        .collect()
}

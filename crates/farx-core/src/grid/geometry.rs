use ratatui::layout::Rect;

/// Maximum number of tiles shown at full size; the rest are minimized.
pub const MAX_FULL_TILES: usize = 6;

/// Pack `count` tiles into a near-square grid within `area`.
///
/// `cols = ceil(sqrt(count))`, `rows = ceil(count / cols)`, filled row-major
/// (top-to-bottom, left-to-right). Row heights split `area.height` evenly
/// (early rows absorb the remainder). Within each row the tiles split that
/// row's width evenly, so a short final row stretches to fill the width.
/// Returns exactly `count` rects; empty when `count == 0`.
pub fn grid_rects(area: Rect, count: usize) -> Vec<Rect> {
    if count == 0 {
        return Vec::new();
    }
    let cols = (count as f64).sqrt().ceil() as usize;
    let rows = count.div_ceil(cols);

    let mut out = Vec::with_capacity(count);
    let mut remaining = count;
    for row in 0..rows {
        let tiles_in_row = remaining.min(cols);
        let y = area.y + span_start(area.height, rows, row);
        let h = span_len(area.height, rows, row);
        for col in 0..tiles_in_row {
            let x = area.x + span_start(area.width, tiles_in_row, col);
            let w = span_len(area.width, tiles_in_row, col);
            out.push(Rect::new(x, y, w, h));
        }
        remaining -= tiles_in_row;
    }
    out
}

/// Offset of slice `index` when dividing `total` into `parts` near-equal
/// spans (earlier spans absorb the remainder, so coverage is exact).
fn span_start(total: u16, parts: usize, index: usize) -> u16 {
    let base = total / parts as u16;
    let rem = total % parts as u16;
    let i = index as u16;
    base * i + i.min(rem)
}

/// Length of slice `index` when dividing `total` into `parts` near-equal
/// spans (earlier spans get one extra unit until the remainder is spent).
fn span_len(total: u16, parts: usize, index: usize) -> u16 {
    let base = total / parts as u16;
    let rem = total % parts as u16;
    base + if (index as u16) < rem { 1 } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::layout::Rect;

    fn area() -> Rect {
        Rect::new(0, 0, 120, 40)
    }

    #[test]
    fn grid_rects_zero_is_empty() {
        assert!(grid_rects(area(), 0).is_empty());
    }

    #[test]
    fn grid_rects_one_fills_area() {
        let r = grid_rects(area(), 1);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0], area());
    }

    #[test]
    fn grid_rects_two_side_by_side() {
        let r = grid_rects(area(), 2);
        assert_eq!(r.len(), 2);
        // One row, two columns: same height as the area, each ~half width.
        assert_eq!(r[0].height, 40);
        assert_eq!(r[1].height, 40);
        assert_eq!(r[0].x, 0);
        assert!(r[1].x >= 59 && r[1].x <= 61);
        // No horizontal overlap.
        assert!(r[0].x + r[0].width <= r[1].x);
    }

    #[test]
    fn grid_rects_four_is_two_by_two() {
        let r = grid_rects(area(), 4);
        assert_eq!(r.len(), 4);
        // Two distinct row y-offsets, two distinct column x-offsets.
        let ys: std::collections::BTreeSet<u16> = r.iter().map(|x| x.y).collect();
        let xs: std::collections::BTreeSet<u16> = r.iter().map(|x| x.x).collect();
        assert_eq!(ys.len(), 2);
        assert_eq!(xs.len(), 2);
    }

    #[test]
    fn grid_rects_six_is_three_cols_two_rows() {
        let r = grid_rects(area(), 6);
        assert_eq!(r.len(), 6);
        let ys: std::collections::BTreeSet<u16> = r.iter().map(|x| x.y).collect();
        let xs: std::collections::BTreeSet<u16> = r.iter().map(|x| x.x).collect();
        assert_eq!(xs.len(), 3); // cols = ceil(sqrt(6)) = 3
        assert_eq!(ys.len(), 2); // rows = ceil(6/3) = 2
    }

    #[test]
    fn grid_rects_three_last_row_stretches_full_width() {
        // cols = ceil(sqrt(3)) = 2, rows = 2. Row 0 has 2 tiles, row 1 has 1.
        let r = grid_rects(area(), 3);
        assert_eq!(r.len(), 3);
        // The lone tile on the last row stretches to the full area width.
        let last = r[2];
        assert_eq!(last.x, 0);
        assert_eq!(last.width, 120);
    }

    #[test]
    fn grid_rects_cover_without_overlap_for_many_counts() {
        for count in 1..=12usize {
            let rects = grid_rects(area(), count);
            assert_eq!(rects.len(), count, "count {count}");
            for (i, a) in rects.iter().enumerate() {
                assert!(a.width > 0 && a.height > 0, "count {count} tile {i} empty");
                for b in rects.iter().skip(i + 1) {
                    let disjoint = a.x + a.width <= b.x
                        || b.x + b.width <= a.x
                        || a.y + a.height <= b.y
                        || b.y + b.height <= a.y;
                    assert!(disjoint, "count {count}: tiles {a:?} and {b:?} overlap");
                }
            }
        }
    }
}

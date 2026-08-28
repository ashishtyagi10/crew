//! Squarified treemap: sizes as areas, packed into a rectangle.
//!
//! A list of directory sizes sorted descending tells you which is biggest and
//! nothing about proportion — 4.2G above 3.9G above 120M reads as three lines
//! until you do the arithmetic. As areas, the same three are a half, a half,
//! and a sliver, and the answer arrives before the numbers do.
//!
//! The layout is Bruls/Huizing/van Wijk squarified: lay items along the short
//! edge of the space left, keeping the worst aspect ratio in the current row
//! as close to 1 as possible, because a tile you can read is one you can point
//! at.
/// A laid-out tile: the item's index in the input, and its rect in the same
/// units the bounding rect was given in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tile {
    pub index: usize,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Tile {
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

/// Lay `values` out in `(x, y, w, h)`. Values are taken in the order given —
/// callers sort descending, which is what makes the layout stable and the big
/// tiles land top-left. Zero and negative values get no tile at all.
pub fn layout(rect: (f32, f32, f32, f32), values: &[f64]) -> Vec<Tile> {
    let (x, y, w, h) = rect;
    let total: f64 = values.iter().filter(|v| **v > 0.0).sum();
    if w <= 0.0 || h <= 0.0 || total <= 0.0 {
        return Vec::new();
    }
    // Work in area units: one unit of value = `scale` units of area.
    let scale = (w as f64 * h as f64) / total;
    let mut out = Vec::new();
    let mut free = (x, y, w, h);
    let mut row: Vec<(usize, f64)> = Vec::new();
    let mut i = 0;
    let items: Vec<(usize, f64)> = values
        .iter()
        .enumerate()
        .filter(|(_, v)| **v > 0.0)
        .map(|(i, v)| (i, *v * scale))
        .collect();

    while i < items.len() {
        let candidate = items[i];
        let short = free.2.min(free.3) as f64;
        if row.is_empty() || worst(&row, short) >= worst_with(&row, candidate.1, short) {
            row.push(candidate);
            i += 1;
            continue;
        }
        // Adding it would make the row's tiles worse-shaped: close the row.
        free = place_row(&mut out, &row, free);
        row.clear();
    }
    if !row.is_empty() {
        place_row(&mut out, &row, free);
    }
    out
}

/// Worst aspect ratio in `row` if laid along an edge of length `short`.
fn worst(row: &[(usize, f64)], short: f64) -> f64 {
    if row.is_empty() || short <= 0.0 {
        return f64::MAX;
    }
    let sum: f64 = row.iter().map(|(_, a)| a).sum();
    if sum <= 0.0 {
        return f64::MAX;
    }
    let (min, max) = row.iter().fold((f64::MAX, 0.0f64), |(lo, hi), (_, a)| {
        (lo.min(*a), hi.max(*a))
    });
    let s2 = short * short;
    ((s2 * max) / (sum * sum)).max((sum * sum) / (s2 * min))
}

fn worst_with(row: &[(usize, f64)], area: f64, short: f64) -> f64 {
    let mut next: Vec<(usize, f64)> = row.to_vec();
    next.push((usize::MAX, area));
    worst(&next, short)
}

/// Place `row` along the short edge of `free`, returning what is left.
fn place_row(
    out: &mut Vec<Tile>,
    row: &[(usize, f64)],
    free: (f32, f32, f32, f32),
) -> (f32, f32, f32, f32) {
    let (x, y, w, h) = free;
    let sum: f64 = row.iter().map(|(_, a)| a).sum();
    if sum <= 0.0 || w <= 0.0 || h <= 0.0 {
        return free;
    }
    if w <= h {
        // Rows run left→right across the top of what is left.
        let band = (sum / w as f64) as f32;
        let mut cx = x;
        for (idx, area) in row {
            let tw = (*area / band as f64) as f32;
            out.push(Tile {
                index: *idx,
                x: cx,
                y,
                w: tw,
                h: band,
            });
            cx += tw;
        }
        (x, y + band, w, h - band)
    } else {
        // Columns run top→bottom down the left of what is left.
        let band = (sum / h as f64) as f32;
        let mut cy = y;
        for (idx, area) in row {
            let th = (*area / band as f64) as f32;
            out.push(Tile {
                index: *idx,
                x,
                y: cy,
                w: band,
                h: th,
            });
            cy += th;
        }
        (x + band, y, w - band, h)
    }
}

#[cfg(test)]
mod tests {
    use super::layout;

    fn area(t: &super::Tile) -> f32 {
        t.w * t.h
    }

    #[test]
    fn a_tiles_area_is_its_share_of_the_rect() {
        let tiles = layout((0.0, 0.0, 10.0, 10.0), &[50.0, 25.0, 25.0]);
        assert_eq!(tiles.len(), 3);
        let total: f32 = tiles.iter().map(area).sum();
        assert!((total - 100.0).abs() < 0.5, "the rect is filled: {total}");
        let half = tiles.iter().find(|t| t.index == 0).unwrap();
        assert!(
            (area(half) - 50.0).abs() < 1.0,
            "half the value is half the area: {}",
            area(half)
        );
    }

    #[test]
    fn tiles_do_not_overlap() {
        let tiles = layout(
            (0.0, 0.0, 12.0, 8.0),
            &[40.0, 22.0, 13.0, 9.0, 8.0, 5.0, 3.0],
        );
        for (i, a) in tiles.iter().enumerate() {
            for b in tiles.iter().skip(i + 1) {
                let sep = a.x + a.w <= b.x + 1e-3
                    || b.x + b.w <= a.x + 1e-3
                    || a.y + a.h <= b.y + 1e-3
                    || b.y + b.h <= a.y + 1e-3;
                assert!(sep, "{a:?} overlaps {b:?}");
            }
        }
    }

    #[test]
    fn every_tile_stays_inside_the_rect() {
        let tiles = layout((2.0, 3.0, 10.0, 6.0), &[9.0, 8.0, 7.0, 1.0, 1.0]);
        for t in &tiles {
            assert!(t.x >= 2.0 - 1e-3 && t.x + t.w <= 12.0 + 1e-2, "{t:?}");
            assert!(t.y >= 3.0 - 1e-3 && t.y + t.h <= 9.0 + 1e-2, "{t:?}");
        }
    }

    #[test]
    fn tiles_are_squarish_rather_than_slivers() {
        // The whole point of squarifying: a naive slice-and-dice layout gives
        // the small items aspect ratios in the tens.
        let vals: Vec<f64> = (1..=12).map(|i| i as f64 * 3.0).collect();
        let tiles = layout((0.0, 0.0, 16.0, 10.0), &vals);
        let worst = tiles
            .iter()
            .map(|t| (t.w / t.h).max(t.h / t.w))
            .fold(0.0f32, f32::max);
        assert!(worst < 5.0, "worst aspect ratio {worst}");
    }

    #[test]
    fn a_value_of_zero_gets_no_tile() {
        let tiles = layout((0.0, 0.0, 8.0, 4.0), &[10.0, 0.0, 5.0]);
        assert_eq!(tiles.len(), 2);
        assert!(tiles.iter().all(|t| t.index != 1));
    }

    #[test]
    fn nothing_to_lay_out_lays_nothing_out() {
        assert!(layout((0.0, 0.0, 8.0, 4.0), &[]).is_empty());
        assert!(layout((0.0, 0.0, 8.0, 4.0), &[0.0, 0.0]).is_empty());
        assert!(layout((0.0, 0.0, 0.0, 4.0), &[1.0]).is_empty());
    }

    #[test]
    fn the_biggest_tile_is_the_one_you_point_at_first() {
        // Descending input puts the largest tile at the origin corner.
        let tiles = layout((0.0, 0.0, 10.0, 10.0), &[60.0, 30.0, 10.0]);
        let first = tiles.iter().find(|t| t.index == 0).unwrap();
        assert!(first.x < 1e-3 && first.y < 1e-3, "{first:?}");
    }

    #[test]
    fn hit_testing_finds_the_tile_under_a_point() {
        let tiles = layout((0.0, 0.0, 10.0, 10.0), &[50.0, 30.0, 20.0]);
        for t in &tiles {
            let (cx, cy) = (t.x + t.w / 2.0, t.y + t.h / 2.0);
            let hit = tiles.iter().filter(|o| o.contains(cx, cy)).count();
            assert_eq!(hit, 1, "one tile owns its own centre");
        }
    }
}

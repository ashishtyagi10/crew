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
#[path = "treemap_tests.rs"]
mod tests;

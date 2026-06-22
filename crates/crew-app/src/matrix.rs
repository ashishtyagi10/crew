//! Matrix "digital rain": per-column streams of glyphs falling down the grid,
//! each with a bright white-green head and a fading green trail. Fully stateless —
//! a frame is a pure function of `(cols, rows, tick)`.
use crew_render::CellView;

const BG: (u8, u8, u8) = (0, 0, 0);
const TRAIL: i64 = 14;
const GAP: i64 = 8;

/// Glyph pool: half-width katakana + a few latin/digits, the classic mix.
const GLYPHS: &[char] = &[
    'ｱ', 'ｲ', 'ｳ', 'ｴ', 'ｵ', 'ｶ', 'ｷ', 'ｸ', 'ｹ', 'ｺ', 'ｻ', 'ｼ', 'ｽ', 'ｾ', 'ｿ', 'ﾀ', 'ﾁ', 'ﾂ', 'ﾃ',
    'ﾄ', 'ﾅ', 'ﾆ', 'ﾇ', 'ﾈ', 'ﾉ', 'ﾊ', 'ﾋ', 'ﾌ', 'ﾍ', 'ﾎ', '0', '1', '3', '5', '7', '9', '=', ':',
    '.', '*', '+',
];

/// A fast integer hash (splitmix64-style) for deterministic per-cell randomness.
fn hash(mut x: u64) -> u64 {
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51afd7ed558ccd);
    x ^= x >> 33;
    x = x.wrapping_mul(0xc4ceb9fe1a85ec53);
    x ^ (x >> 33)
}

fn glyph(col: u16, row: u16, salt: u64) -> char {
    let h = hash(((col as u64) << 32) ^ ((row as u64) << 8) ^ salt);
    GLYPHS[(h as usize) % GLYPHS.len()]
}

/// Colour for a cell `d` rows behind the bright head (d == 0 is the head).
fn fade(d: i64) -> ((u8, u8, u8), bool) {
    if d == 0 {
        return ((200, 255, 210), true); // white-green head, bold
    }
    let t = d as f32 / TRAIL as f32;
    let g = (255.0 * (1.0 - t)).max(45.0) as u8;
    ((0, g, g / 6), false)
}

/// Render one frame of digital rain into a `cols × rows` grid.
pub fn rain(cols: u16, rows: u16, tick: u64) -> Vec<CellView> {
    if cols == 0 || rows == 0 {
        return Vec::new();
    }
    let cycle = rows as i64 + TRAIL + GAP;
    let mut cells = Vec::new();
    for c in 0..cols {
        let seed = hash(c as u64 + 1);
        let period = 12 + (seed % 11); // ticks per cell of fall (12..=22) — a slow drift
        let phase = ((seed >> 8) as i64).rem_euclid(cycle);
        let head = (((tick / period) as i64) + phase).rem_euclid(cycle);
        for d in 0..TRAIL {
            let r = head - d;
            if r < 0 || r >= rows as i64 {
                continue;
            }
            // Glyphs flicker; the head churns fastest, the trail much slower.
            let salt = tick / if d == 0 { 2 } else { 11 };
            let (fg, bold) = fade(d);
            cells.push(CellView {
                col: c,
                row: r as u16,
                c: glyph(c, r as u16, salt),
                fg,
                bg: BG,
                bold,
                italic: false,
            });
        }
    }
    cells
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rain_is_in_bounds_and_animates() {
        let a = rain(40, 20, 0);
        assert!(a.iter().all(|c| c.col < 40 && c.row < 20));
        assert!(!a.is_empty());
        // a later frame differs (the rain has moved)
        let b = rain(40, 20, 30);
        assert_ne!(
            a.iter().map(|c| (c.col, c.row)).collect::<Vec<_>>(),
            b.iter().map(|c| (c.col, c.row)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn has_one_bright_head_per_active_column() {
        let cells = rain(10, 30, 5);
        assert!(cells.iter().any(|c| c.fg == (200, 255, 210) && c.bold));
    }

    #[test]
    fn zero_size_is_empty() {
        assert!(rain(0, 10, 1).is_empty());
        assert!(rain(10, 0, 1).is_empty());
    }
}
